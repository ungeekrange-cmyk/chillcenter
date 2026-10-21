//! Read-only release discovery for legacy installations without a root helper.
//! Separate state is deliberate: these UI hints never authorize installation or
//! consume the privileged helper's pending intents/replay ledger.
use chill_updates::{
    Component, GithubReleaseClient, IntentKind, Operation, OperationStatus, RuntimeCompatibility,
    StateStore, TransportLimits, UpdateError, VerificationContext,
};
use semver::Version;
use serde_json::{Value, json};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};
use time::OffsetDateTime;

const PUBLIC_KEY: &str = include_str!("../../../updates/release.pub");
const KEY_ID: &str = "18FD790B48C202F8";
const COMPONENTS: [Component; 3] = [Component::Server, Component::Web, Component::Themes];
// At most one network worker per component, never an unbounded task queue.
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
pub(super) fn busy() -> bool {
    ACTIVE.load(Ordering::Acquire) != 0
}
struct Lease(usize);
impl Drop for Lease {
    fn drop(&mut self) {
        ACTIVE.fetch_and(!self.0, Ordering::AcqRel);
    }
}
fn bit(component: Component) -> chill_updates::Result<usize> {
    COMPONENTS
        .iter()
        .position(|value| *value == component)
        .map(|index| 1 << index)
        .ok_or_else(|| UpdateError::Invalid("server component required".into()))
}

pub(super) fn admit(path: PathBuf, component: Component) -> chill_updates::Result<Value> {
    let mask = bit(component)?;
    let store = StateStore::open(&path)?;
    if ACTIVE.fetch_or(mask, Ordering::AcqRel) & mask != 0 {
        return store
            .read()?
            .operations
            .into_iter()
            .rev()
            .find(|op| op.component == component)
            .map(|operation| json!({"operation":operation,"cached":true}))
            .ok_or_else(|| UpdateError::Busy("check admission in progress".into()));
    }
    let lease = Lease(mask);
    let operation = store.admit_intent(component, IntentKind::Check, OffsetDateTime::now_utc())?;
    if operation.status == OperationStatus::Running {
        // A process restart lost this worker; do not leave a permanent spinner.
        let operation = store.transition(
            operation.id,
            OperationStatus::Running,
            OperationStatus::Failed,
            Some("CHECK_INTERRUPTED"),
            OffsetDateTime::now_utc(),
        )?;
        return Ok(json!({"operation":operation,"cached":true}));
    }
    if operation.status != OperationStatus::Pending {
        return Ok(json!({"operation":operation,"cached":true}));
    }
    let response = json!({"operation":operation});
    tokio::task::spawn_blocking(move || {
        let _lease = lease;
        let _ = complete(&path, &operation, || fetch(component));
    });
    Ok(response)
}

fn complete(
    path: &Path,
    operation: &Operation,
    fetch: impl FnOnce() -> chill_updates::Result<Option<Value>>,
) -> chill_updates::Result<()> {
    let store = StateStore::open(path)?;
    store.transition(
        operation.id,
        OperationStatus::Pending,
        OperationStatus::Running,
        None,
        OffsetDateTime::now_utc(),
    )?;
    // No state/catalogue lock is held across HTTP/signature verification.
    let outcome = fetch().and_then(|value| {
        publish(path, operation.component, value.as_ref())?;
        Ok(value)
    });
    let (status, code) = match outcome {
        Ok(Some(_)) => (OperationStatus::Succeeded, "AVAILABLE"),
        Ok(None) => (OperationStatus::Succeeded, "NO_UPDATE"),
        Err(UpdateError::Incompatible(_)) => (OperationStatus::Failed, "CHECK_INCOMPATIBLE"),
        Err(_) => (OperationStatus::Failed, "CHECK_FAILED"),
    };
    store.transition(
        operation.id,
        OperationStatus::Running,
        status,
        Some(code),
        OffsetDateTime::now_utc(),
    )?;
    Ok(())
}

fn publish(path: &Path, component: Component, value: Option<&Value>) -> chill_updates::Result<()> {
    bit(component)?;
    let temporary = path.join(format!(".check-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&temporary)?;
        file.write_all(&serde_json::to_vec(&value)?)?;
        file.sync_all()?;
        fs::rename(
            &temporary,
            path.join(format!("{}.json", component.as_str())),
        )?;
        fs::File::open(path)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

pub(super) fn snapshot(path: &Path) -> chill_updates::Result<(Vec<Operation>, Option<Value>)> {
    let operations = StateStore::open(path)?.read()?.operations;
    let mut available = serde_json::Map::new();
    for component in COMPONENTS {
        let latest = operations.iter().rev().find(|op| op.component == component);
        if latest.is_none_or(|op| {
            op.status != OperationStatus::Succeeded
                || op.result_code.as_deref() != Some("AVAILABLE")
        }) {
            continue;
        }
        if let Ok(file) = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path.join(format!("{}.json", component.as_str())))
        {
            if !file.metadata()?.is_file() || file.metadata()?.len() > 64 * 1024 {
                continue;
            }
            let mut bytes = Vec::new();
            file.take(64 * 1024 + 1).read_to_end(&mut bytes)?;
            if bytes.len() <= 64 * 1024
                && let Ok(value @ Value::Object(_)) = serde_json::from_slice::<Value>(&bytes)
            {
                available.insert(component.as_str().into(), value);
            }
        }
    }
    Ok((operations, Some(Value::Object(available))))
}

fn installed_version(installed: &Value, component: Component) -> chill_updates::Result<Version> {
    installed[component.as_str()]["version"]
        .as_str()
        .and_then(|v| Version::parse(v).ok())
        .ok_or_else(|| UpdateError::Invalid("installed component version unavailable".into()))
}

fn fetch(component: Component) -> chill_updates::Result<Option<Value>> {
    bit(component)?;
    let installed = super::installed_versions();
    let current = installed_version(&installed, component)?;
    let client = GithubReleaseClient::new(TransportLimits {
        max_manifest_bytes: 64 * 1024,
        max_signature_bytes: 8 * 1024,
        timeout: std::time::Duration::from_secs(15),
        ..Default::default()
    })?;
    let Some(candidate) = client.find_latest(component)? else {
        return Ok(None);
    };
    if candidate.version <= current {
        return Ok(None);
    }
    let (bytes, signature) = client.download_manifest(&candidate)?;
    let target = match component {
        Component::Web => "web",
        Component::Themes => "universal",
        Component::Server if cfg!(target_arch = "aarch64") => "linux-aarch64",
        Component::Server if cfg!(target_arch = "arm") => "linux-armv7",
        _ => {
            return Err(UpdateError::Incompatible(
                "no published server target".into(),
            ));
        }
    };
    let context = VerificationContext {
        now: OffsetDateTime::now_utc(),
        expected_key_id: KEY_ID,
        component,
        target,
        installed_version: &current,
        // Discovery never accepts a release or changes the root replay ledger.
        // Installation independently repeats signature/sequence/compatibility checks.
        highest_sequence: 0,
        runtime: RuntimeCompatibility {
            api_version: Some(Version::new(2026, 10, 14)),
            server_version: Some(installed_version(&installed, Component::Server)?),
            database_schema: Some(3),
            renderer_version: installed["themes"]["renderer_version"]
                .as_u64()
                .and_then(|v| v.try_into().ok()),
            theme_format: installed["themes"]["format_version"]
                .as_u64()
                .and_then(|v| v.try_into().ok()),
        },
    };
    let verified =
        chill_updates::parse_and_verify_manifest(&bytes, &signature, PUBLIC_KEY, &context)?;
    if verified.manifest.release_tag != candidate.tag
        || verified.manifest.version != candidate.version
    {
        return Err(UpdateError::Security(
            "release and signed manifest differ".into(),
        ));
    }
    Ok(Some(
        json!({"version":verified.manifest.version,"source_commit":verified.manifest.source_commit,
        "release_sequence":verified.manifest.release_sequence,"checked_unix":context.now.unix_timestamp()}),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn passive_checks_report_results_without_helper_or_install_intents() {
        let dir = tempfile::tempdir().unwrap();
        let store = StateStore::open(dir.path()).unwrap();
        let op = store
            .admit_intent(Component::Web, IntentKind::Check, OffsetDateTime::now_utc())
            .unwrap();
        complete(dir.path(), &op, || Ok(Some(json!({"version":"0.7.2"})))).unwrap();
        let (operations, available) = snapshot(dir.path()).unwrap();
        assert_eq!(operations[0].result_code.as_deref(), Some("AVAILABLE"));
        assert_eq!(available.unwrap()["web"]["version"], "0.7.2");
        assert!(operations.iter().all(|op| op.kind == IntentKind::Check));
        let cached = store
            .admit_intent(Component::Web, IntentKind::Check, OffsetDateTime::now_utc())
            .unwrap();
        assert_eq!(cached.id, op.id);
        assert!(bit(Component::Mac).is_err());
    }
    #[test]
    fn failure_or_no_update_cannot_leave_an_installable_stale_hint() {
        for outcome in [
            Ok(None),
            Err(UpdateError::Security("invalid signature".into())),
            Err(UpdateError::Incompatible("renderer".into())),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let store = StateStore::open(dir.path()).unwrap();
            publish(
                dir.path(),
                Component::Themes,
                Some(&json!({"version":"9.0.0"})),
            )
            .unwrap();
            let op = store
                .admit_intent(
                    Component::Themes,
                    IntentKind::Check,
                    OffsetDateTime::now_utc(),
                )
                .unwrap();
            complete(dir.path(), &op, || outcome).unwrap();
            assert_eq!(snapshot(dir.path()).unwrap().1.unwrap(), json!({}));
        }
    }
    #[test]
    fn status_remains_readable_while_a_metadata_fetch_is_waiting() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_path_buf();
        let store = StateStore::open(&path).unwrap();
        let operation = store
            .admit_intent(Component::Web, IntentKind::Check, OffsetDateTime::now_utc())
            .unwrap();
        let (entered, waiting) = std::sync::mpsc::channel();
        let (finish, released) = std::sync::mpsc::channel();
        let worker_path = path.clone();
        let worker = std::thread::spawn(move || {
            complete(&worker_path, &operation, || {
                entered.send(()).unwrap();
                released
                    .recv_timeout(std::time::Duration::from_secs(5))
                    .unwrap();
                Ok(None)
            })
        });
        waiting
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        let reading = std::thread::spawn(move || snapshot(&path));
        // Fetch must remain blocked until after a completely separate status read.
        let status = reading.join().unwrap().unwrap();
        finish.send(()).unwrap();
        worker.join().unwrap().unwrap();
        assert_eq!(status.0[0].status, OperationStatus::Running);
    }
}
