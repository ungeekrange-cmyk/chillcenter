//! Explicit Mac-only CLI proposals. Nothing is applied until a captured plan is confirmed.
use chill_catalog::{
    assistant,
    model::{Media, Settings},
};
use chill_client::{
    HttpMethod,
    ui::{PlaybackConnection, UiError},
};
use chill_domain::ai::AiContext;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum AiCommand {
    Settings {},
    Configure {
        provider: String,
        codex_path: String,
        claude_path: String,
    },
    Start {
        request: String,
        mode: String,
        category: Option<String>,
        q: Option<String>,
    },
    Status {
        id: String,
    },
    Cancel {
        id: String,
    },
    Apply {
        id: String,
        confirmed: bool,
    },
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CliSettings {
    provider: String,
    codex_path: String,
    claude_path: String,
}
impl Default for CliSettings {
    fn default() -> Self {
        Self {
            provider: "codex".into(),
            codex_path: "codex".into(),
            claude_path: "claude".into(),
        }
    }
}
impl CliSettings {
    fn validate(&self) -> Result<(), UiError> {
        if !["codex", "claude"].contains(&self.provider.as_str())
            || [&self.codex_path, &self.claude_path].iter().any(|p| {
                p.is_empty()
                    || p.len() > 1024
                    || p.chars().any(char::is_control)
                    || (!PathBuf::from(p).is_absolute()
                        && !["codex", "claude"].contains(&p.as_str()))
            })
        {
            return Err(error(
                "Choisissez Codex ou Claude et un chemin absolu vers leur CLI.",
                "AI_SETTINGS_INVALID",
            ));
        }
        Ok(())
    }
    fn provider_settings(&self) -> Settings {
        Settings {
            ai_provider: self.provider.clone(),
            codex_path: self.codex_path.clone(),
            claude_path: self.claude_path.clone(),
            ..Settings::default()
        }
    }
}
pub struct AiHost {
    directory: PathBuf,
    settings: Mutex<CliSettings>,
    job: Mutex<Option<Arc<Mutex<Job>>>>,
}
pub struct Job {
    pub connection: PlaybackConnection,
    id: String,
    stage: &'static str,
    mode: String,
    context: Option<AiContext>,
    plan: Option<assistant::Plan>,
    result: Option<Value>,
    message: Option<String>,
    cancelled: Arc<AtomicBool>,
    wake: Arc<tokio::sync::Notify>,
    created: Instant,
}
pub fn error(message: &str, code: &str) -> UiError {
    UiError {
        message: message.into(),
        status: None,
        code: code.into(),
    }
}
fn lock_error() -> UiError {
    error("Assistant indisponible.", "AI_BUSY")
}
impl AiHost {
    pub fn open(directory: PathBuf) -> Result<Self, UiError> {
        let path = directory.join("ai-settings.json");
        let settings = match OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(path)
        {
            Ok(file) => {
                if !file.metadata().map_err(|_| lock_error())?.is_file() {
                    return Err(lock_error());
                }
                let mut bytes = Vec::new();
                file.take(4097)
                    .read_to_end(&mut bytes)
                    .map_err(|_| lock_error())?;
                if bytes.len() > 4096 {
                    return Err(lock_error());
                }
                serde_json::from_slice::<CliSettings>(&bytes).map_err(|_| lock_error())?
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => CliSettings::default(),
            Err(_) => return Err(lock_error()),
        };
        settings.validate()?;
        Ok(Self {
            directory,
            settings: Mutex::new(settings),
            job: Mutex::new(None),
        })
    }
    pub fn settings(&self) -> Result<Value, UiError> {
        Ok(json!(*self.settings.lock().map_err(|_| lock_error())?))
    }
    pub fn configure(
        &self,
        provider: String,
        codex_path: String,
        claude_path: String,
    ) -> Result<Value, UiError> {
        if self
            .job
            .lock()
            .map_err(|_| lock_error())?
            .as_ref()
            .is_some_and(|j| {
                j.lock()
                    .is_ok_and(|j| ["working", "applying"].contains(&j.stage))
            })
        {
            return Err(error("Attendez ou annulez la demande en cours.", "AI_BUSY"));
        }
        let next = CliSettings {
            provider,
            codex_path,
            claude_path,
        };
        next.validate()?;
        let bytes = serde_json::to_vec(&next).map_err(|_| lock_error())?;
        let temp = self
            .directory
            .join(format!(".ai-settings-{}.tmp", uuid::Uuid::new_v4()));
        let write = (|| -> std::io::Result<()> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&temp)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            fs::rename(&temp, self.directory.join("ai-settings.json"))?;
            fs::File::open(&self.directory)?.sync_all()
        })();
        if write.is_err() {
            let _ = fs::remove_file(temp);
            return Err(error(
                "Réglages non enregistrés. Vérifiez le stockage du Mac.",
                "AI_STORAGE",
            ));
        }
        *self.settings.lock().map_err(|_| lock_error())? = next;
        self.settings()
    }
    pub fn get(&self, id: &str, current: &PlaybackConnection) -> Result<Arc<Mutex<Job>>, UiError> {
        let job = self
            .job
            .lock()
            .map_err(|_| lock_error())?
            .as_ref()
            .cloned()
            .ok_or_else(|| error("Proposition expirée.", "AI_NOT_FOUND"))?;
        {
            let j = job.lock().map_err(|_| lock_error())?;
            if j.id != id || j.created.elapsed() > Duration::from_secs(900) {
                return Err(error("Proposition expirée.", "AI_NOT_FOUND"));
            }
            if j.connection.server_id != current.server_id
                || j.connection.profile_id != current.profile_id
            {
                return Err(error(
                    "Le profil ou le serveur a changé. Relancez explicitement l’IA.",
                    "AI_STALE",
                ));
            }
        }
        Ok(job)
    }
    pub fn cancel_all(&self) {
        if let Ok(job) = self.job.lock()
            && let Some(job) = job.as_ref() {
                let _ = cancel(job);
        }
    }
    pub fn start(
        &self,
        connection: PlaybackConnection,
        request: String,
        mode: String,
        category: Option<String>,
        q: Option<String>,
    ) -> Result<Value, UiError> {
        if !["organize", "search"].contains(&mode.as_str())
            || request.trim().is_empty()
            || request.len() > 8000
            || category
                .as_ref()
                .is_some_and(|c| !chill_catalog::model::CATEGORIES.contains(&c.as_str()))
            || q.as_ref().is_some_and(|q| q.len() > 500)
        {
            return Err(error(
                "Demande invalide ou trop longue.",
                "AI_REQUEST_INVALID",
            ));
        }
        let mut active = self.job.lock().map_err(|_| lock_error())?;
        if active.as_ref().is_some_and(|j| {
            j.lock()
                .is_ok_and(|j| ["working", "applying"].contains(&j.stage))
        }) {
            return Err(error("Une demande IA est déjà en cours.", "AI_BUSY"));
        }
        let settings = self.settings.lock().map_err(|_| lock_error())?.clone();
        let job = Arc::new(Mutex::new(Job {
            connection,
            id: uuid::Uuid::new_v4().to_string(),
            stage: "working",
            mode: mode.clone(),
            context: None,
            plan: None,
            result: None,
            message: None,
            cancelled: Arc::new(AtomicBool::new(false)),
            wake: Arc::new(tokio::sync::Notify::new()),
            created: Instant::now(),
        }));
        *active = Some(job.clone());
        let out = view(&job)?;
        tauri::async_runtime::spawn(settle_proposal(
            job.clone(),
            propose(job.clone(), settings, request, mode, category, q),
            Duration::from_secs(180),
        ));
        Ok(out)
    }
}
async fn settle_proposal<F>(job: Arc<Mutex<Job>>, work: F, deadline: Duration)
where
    F: std::future::Future<Output = Result<(AiContext, assistant::Plan), UiError>>,
{
    let (flag, wake) = match job.lock() {
        Ok(j) => (j.cancelled.clone(), j.wake.clone()),
        Err(_) => return,
    };
    let outcome = tokio::select! {biased;
        _ = wake.notified() => Err(error("Demande annulée.", "AI_CANCELLED")),
        result = tokio::time::timeout(deadline, work) => result.unwrap_or_else(|_| Err(error("Délai IA dépassé. Aucune modification appliquée.", "AI_TIMEOUT"))),
    };
    if let Ok(mut j) = job.lock() {
        if flag.load(Ordering::Acquire) {
            j.stage = "cancelled";
            j.plan = None;
            j.context = None;
            return;
        }
        match outcome {
            Ok((context, plan)) => {
                j.context = Some(context);
                j.plan = Some(plan);
                j.stage = "preview";
            }
            Err(e) => {
                j.stage = "failed";
                j.message = Some(e.message);
                j.plan = None;
                j.context = None;
            }
        }
    }
}
pub fn view(job: &Arc<Mutex<Job>>) -> Result<Value, UiError> {
    let j = job.lock().map_err(|_| lock_error())?;
    let items: Vec<Value> = j
        .context
        .as_ref()
        .map(|c| {
            c.media
                .iter()
                .map(|m| json!({"id":m.id,"title":m.title}))
                .collect()
        })
        .unwrap_or_default();
    Ok(
        json!({"id":j.id,"stage":j.stage,"mode":j.mode,"plan":j.plan,"items":items,"result":j.result,"message":j.message}),
    )
}
pub fn cancel(job: &Arc<Mutex<Job>>) -> Result<Value, UiError> {
    {
        let mut j = job.lock().map_err(|_| lock_error())?;
        if j.stage == "applying" {
            return Err(error(
                "Application déjà confirmée : attendez son résultat.",
                "AI_CONFIRMED",
            ));
        }
        j.cancelled.store(true, Ordering::Release);
        j.wake.notify_one();
        j.stage = "cancelled";
        j.plan = None;
        j.context = None;
    }
    view(job)
}
fn catalogue(context: &AiContext) -> Vec<Media> {
    context
        .media
        .iter()
        .map(|m| Media {
            id: m.id.clone(),
            path: String::new(),
            title: m.title.clone(),
            category: m.category.clone(),
            year: m.year,
            series: m.series.clone(),
            season: m.season,
            episode: m.episode,
            genres: m.genres.clone(),
            overview: String::new(),
            poster: None,
            poster_custom: false,
            runtime_minutes: None,
            cast: vec![],
            directors: vec![],
            series_overview: String::new(),
            series_metadata: None,
            series_manual: false,
            rating: m.rating,
            saga: m.saga.clone(),
            watched: m.watched,
            available: false,
            added_at: 0,
            tmdb_id: None,
        })
        .collect()
}
struct Workspace(PathBuf);
impl Workspace {
    fn new() -> Result<Self, UiError> {
        use std::os::unix::fs::DirBuilderExt;
        let path = std::env::temp_dir().join(format!("chill-ai-{}", uuid::Uuid::new_v4()));
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .map_err(|_| error("Cache IA indisponible.", "AI_STORAGE"))?;
        Ok(Self(path))
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir(&self.0);
    }
} // Never recursively remove unexpected provider files.
async fn propose(
    job: Arc<Mutex<Job>>,
    settings: CliSettings,
    request: String,
    mode: String,
    category: Option<String>,
    q: Option<String>,
) -> Result<(AiContext, assistant::Plan), UiError> {
    let (transport, profile) = {
        let j = job.lock().map_err(|_| lock_error())?;
        (
            j.connection.transport.clone(),
            j.connection.profile_id.clone(),
        )
    };
    let mut query = Vec::new();
    if let Some(c) = category {
        query.push(("category", c));
    }
    if let Some(q) = q {
        query.push(("q", q));
    }
    let context: AiContext = transport
        .json_with_options(
            HttpMethod::GET,
            &format!("/api/v1/profiles/{profile}/ai/context"),
            None::<&Value>,
            &query,
            &[],
        )
        .await?;
    if context.profile_id != profile
        || context.media.len() > 3000
        || serde_json::to_vec(&context)
            .map_err(|_| lock_error())?
            .len()
            > 1024 * 1024
    {
        return Err(error(
            "Contexte IA invalide ou trop volumineux.",
            "AI_CONTEXT_INVALID",
        ));
    }
    let workspace = Workspace::new()?;
    let catalogue = catalogue(&context);
    let request = if mode == "search" {
        format!("Recherche seulement, aucun changement autorisé. {request}")
    } else {
        request
    };
    let plan = assistant::propose(
        &settings.provider_settings(),
        &catalogue,
        &request,
        &workspace.0,
    )
    .await
    .map_err(|message| error(&message, "AI_PROVIDER_FAILED"))?;
    validate_plan(&plan, &catalogue, &mode)?;
    Ok((context, plan))
}
fn validate_plan(plan: &assistant::Plan, catalogue: &[Media], mode: &str) -> Result<(), UiError> {
    plan.validate(catalogue)
        .map_err(|m| error(&m, "AI_PLAN_INVALID"))?;
    if mode == "search" && !plan.changes.is_empty() {
        return Err(error(
            "Une recherche IA ne peut modifier les données.",
            "AI_PLAN_INVALID",
        ));
    }
    // The approved CLI contract excludes long provider fields and arbitrary tool output.
    for c in &plan.changes {
        let p = &c.patch;
        if p.overview.is_some()
            || p.runtime_minutes.is_some()
            || p.cast.is_some()
            || p.directors.is_some()
            || p.series_overview.is_some()
        {
            return Err(error("Champs IA non autorisés.", "AI_PLAN_INVALID"));
        }
    }
    Ok(())
}
pub fn apply(job: Arc<Mutex<Job>>, confirmed: bool) -> Result<Value, UiError> {
    if !confirmed {
        return Err(error(
            "Confirmez explicitement la proposition affichée.",
            "AI_CONFIRMATION_REQUIRED",
        ));
    }
    let (transport, profile, body) = {
        let mut j = job.lock().map_err(|_| lock_error())?;
        if j.stage != "preview" || j.mode != "organize" || j.cancelled.load(Ordering::Acquire) {
            return Err(error("Proposition indisponible.", "AI_STALE"));
        }
        let context = j.context.as_ref().ok_or_else(lock_error)?;
        let plan = j.plan.as_ref().ok_or_else(lock_error)?;
        validate_plan(plan, &catalogue(context), &j.mode)?;
        let changes: Vec<Value> = plan
            .changes
            .iter()
            .map(|c| {
                let revision = context
                    .media
                    .iter()
                    .find(|m| m.id == c.id)
                    .expect("validated plan")
                    .revision;
                json!({"id":c.id,"expected_revision":revision,"patch":c.patch})
            })
            .collect();
        let body = json!({"expected_common_revision":context.common_revision,"expected_personal_revision":context.personal_revision,"confirmed":true,"changes":changes});
        if serde_json::to_vec(&body).map_err(|_| lock_error())?.len() > 1024 * 1024 {
            return Err(error("Proposition trop volumineuse.", "AI_PLAN_INVALID"));
        }
        let out = (
            j.connection.transport.clone(),
            j.connection.profile_id.clone(),
            body,
        );
        j.stage = "applying";
        out
    };
    let out = view(&job)?;
    tauri::async_runtime::spawn(async move {
        let result = transport
            .request_json::<_, Value>(
                HttpMethod::POST,
                &format!("/api/v1/profiles/{profile}/ai/apply"),
                Some(&body),
            )
            .await;
        if let Ok(mut j) = job.lock() {
            match result {
                Ok(value) => {
                    j.stage = "applied";
                    j.result = Some(value);
                    j.plan = None;
                    j.context = None;
                }
                Err(e) => {
                    j.stage = "failed";
                    j.message = Some(UiError::from(e).message);
                    j.plan = None;
                    j.context = None;
                }
            }
        }
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chill_client::{ServerEndpoint, Transport, TrustStore};
    use std::sync::RwLock;

    fn test_context() -> AiContext {
        AiContext {
            profile_id: "profile".into(),
            common_revision: 1,
            personal_revision: 1,
            media: vec![],
        }
    }
    fn test_plan() -> assistant::Plan {
        assistant::Plan {
            message: "fixture".into(),
            changes: vec![],
            matched_ids: vec![],
        }
    }
    fn test_job() -> Arc<Mutex<Job>> {
        let transport = Transport::new(
            ServerEndpoint::parse("fixture", "https://fixture.invalid").unwrap(),
            Arc::new(RwLock::new(TrustStore::default())),
            Some(Duration::from_millis(1)),
        )
        .unwrap();
        Arc::new(Mutex::new(Job {
            connection: PlaybackConnection {
                transport,
                profile_id: "profile".into(),
                server_id: "fixture".into(),
                device_id: "device".into(),
            },
            id: "job".into(),
            stage: "working",
            mode: "organize".into(),
            context: None,
            plan: None,
            result: None,
            message: None,
            cancelled: Arc::new(AtomicBool::new(false)),
            wake: Arc::new(tokio::sync::Notify::new()),
            created: Instant::now(),
        }))
    }
    #[test]
    fn commands_and_settings_are_closed() {
        for raw in [
            r#"{"action":"settings","path":"/etc/passwd"}"#,
            r#"{"action":"apply","id":"a","confirmed":true,"changes":[]}"#,
        ] {
            assert!(serde_json::from_str::<AiCommand>(raw).is_err());
        }
        assert!(
            CliSettings {
                provider: "shell".into(),
                ..Default::default()
            }
            .validate()
            .is_err()
        );
        assert!(
            CliSettings {
                codex_path: "codex; rm".into(),
                ..Default::default()
            }
            .validate()
            .is_err()
        );
    }
    #[test]
    fn settings_are_private_atomic_and_roundtrip() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let host = AiHost::open(temp.path().into()).unwrap();
        host.configure("claude".into(), "codex".into(), "claude".into())
            .unwrap();
        assert_eq!(
            fs::metadata(temp.path().join("ai-settings.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        assert_eq!(
            AiHost::open(temp.path().into())
                .unwrap()
                .settings()
                .unwrap()["provider"],
            "claude"
        );
    }
    #[test]
    fn search_and_unknown_changes_are_rejected() {
        let plan: assistant::Plan = serde_json::from_value(
            json!({"message":"test","changes":[{"id":"unknown","watched":true}],"matched_ids":[]}),
        )
        .unwrap();
        assert!(validate_plan(&plan, &[], "search").is_err());
    }
    #[test]
    fn cancellation_settles_pending_work_without_a_preview() {
        let job = test_job();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(async {
            let task = tokio::spawn(settle_proposal(
                job.clone(),
                std::future::pending::<Result<(AiContext, assistant::Plan), UiError>>(),
                Duration::from_secs(1),
            ));
            let cancelled = cancel(&job).unwrap();
            assert_eq!(cancelled["stage"], "cancelled");
            task.await.unwrap();
        });
        let j = job.lock().unwrap();
        assert_eq!(j.stage, "cancelled");
        assert!(j.context.is_none() && j.plan.is_none());
    }
    #[test]
    fn timeout_settles_without_a_plan() {
        let job = test_job();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(settle_proposal(
            job.clone(),
            std::future::pending::<Result<(AiContext, assistant::Plan), UiError>>(),
            Duration::from_millis(1),
        ));
        let j = job.lock().unwrap();
        assert_eq!(j.stage, "failed");
        assert!(j.context.is_none() && j.plan.is_none());
        assert_eq!(
            j.message.as_deref(),
            Some("Délai IA dépassé. Aucune modification appliquée.")
        );
    }
    #[test]
    fn late_success_after_cancellation_never_becomes_preview() {
        let job = test_job();
        let wake = Arc::new(tokio::sync::Notify::new());
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(async {
            let work_wake = wake.clone();
            let task = tokio::spawn(settle_proposal(
                job.clone(),
                async move {
                    work_wake.notified().await;
                    Ok((test_context(), test_plan()))
                },
                Duration::from_secs(1),
            ));
            cancel(&job).unwrap();
            wake.notify_one();
            task.await.unwrap();
        });
        let j = job.lock().unwrap();
        assert_eq!(j.stage, "cancelled");
        assert!(j.context.is_none() && j.plan.is_none());
    }
    #[test]
    fn cancellation_during_apply_is_confirmed_and_preserves_state() {
        let job = test_job();
        {
            let mut j = job.lock().unwrap();
            j.stage = "applying";
            j.context = Some(test_context());
            j.plan = Some(test_plan());
        }
        let rejected = cancel(&job).unwrap_err();
        assert_eq!(rejected.code, "AI_CONFIRMED");
        let j = job.lock().unwrap();
        assert_eq!(j.stage, "applying");
        assert!(j.context.is_some() && j.plan.is_some());
    }
}
