use chill_client::ui::{UiClient, UiCommand, UiError};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;
mod admin_bridge;
mod ai_bridge;
mod artwork_bridge;
mod native_player;
mod player_bridge;
use player_bridge::{PlayerCommand, PlayerHost};

#[derive(Clone)]
struct Bridge {
    client: Arc<Mutex<UiClient>>,
    slot: Arc<tokio::sync::Semaphore>,
    player: Arc<tokio::sync::Mutex<PlayerHost>>,
    closing: Arc<std::sync::atomic::AtomicBool>,
    rendering: Arc<std::sync::atomic::AtomicBool>,
    administration: Arc<admin_bridge::AdminHost>,
    assistant: Arc<ai_bridge::AiHost>,
}
#[tauri::command]
async fn ui_command(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, Bridge>,
    command: UiCommand,
) -> Result<Value, UiError> {
    let bridge = state.inner().clone();
    let permit = bridge
        .slot
        .clone()
        .try_acquire_owned()
        .map_err(|_| UiError {
            message: "Une opération est déjà en cours.".into(),
            status: None,
            code: "CLIENT_BUSY".into(),
        })?;
    if matches!(
        &command,
        UiCommand::Connect { .. }
            | UiCommand::Disconnect
            | UiCommand::SelectProfile { .. }
            | UiCommand::DeleteProfile { .. }
            | UiCommand::SaveLibrary { .. }
    ) {
        bridge.administration.cancel_all();
        bridge.assistant.cancel_all();
        bridge.player.lock().await.stop(&window).await?;
    }
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        bridge
            .client
            .lock()
            .map_err(|_| UiError {
                message: "Client indisponible.".into(),
                status: None,
                code: "CLIENT_UNAVAILABLE".into(),
            })?
            .dispatch(command)
    })
    .await
    .map_err(|_| UiError {
        message: "Opération interrompue.".into(),
        status: None,
        code: "CLIENT_INTERRUPTED".into(),
    })?
}
#[tauri::command]
async fn ai_command(
    state: tauri::State<'_, Bridge>,
    command: ai_bridge::AiCommand,
) -> Result<Value, UiError> {
    use ai_bridge::AiCommand as C;
    let bridge = state.inner().clone();
    match command {
        C::Settings {} => return bridge.assistant.settings(),
        C::Configure {
            provider,
            codex_path,
            claude_path,
        } => {
            return bridge
                .assistant
                .configure(provider, codex_path, claude_path);
        }
        _ => {}
    }
    let permit = bridge
        .slot
        .clone()
        .try_acquire_owned()
        .map_err(|_| ai_bridge::error("Une opération est déjà en cours.", "CLIENT_BUSY"))?;
    let client = bridge.client.clone();
    let current = tauri::async_runtime::spawn_blocking(move || {
        client
            .lock()
            .map_err(|_| ai_bridge::error("Client indisponible.", "AI_BUSY"))?
            .playback_connection()
    })
    .await
    .map_err(|_| ai_bridge::error("Client interrompu.", "CLIENT_INTERRUPTED"))??;
    let _permit = permit; // Register/check the job before a profile switch can invalidate it.
    match command {
        C::Start {
            request,
            mode,
            category,
            q,
        } => bridge.assistant.start(current, request, mode, category, q),
        C::Status { id } => ai_bridge::view(&bridge.assistant.get(&id, &current)?),
        C::Cancel { id } => ai_bridge::cancel(&bridge.assistant.get(&id, &current)?),
        C::Apply { id, confirmed } => {
            ai_bridge::apply(bridge.assistant.get(&id, &current)?, confirmed)
        }
        _ => unreachable!(),
    }
}
#[tauri::command]
async fn player_command(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, Bridge>,
    command: PlayerCommand,
) -> Result<Value, UiError> {
    let bridge = state.inner().clone();
    let _permit = bridge
        .slot
        .clone()
        .try_acquire_owned()
        .map_err(|_| UiError {
            message: "Une opération est déjà en cours.".into(),
            status: None,
            code: "CLIENT_BUSY".into(),
        })?;
    if let PlayerCommand::Start {
        media_id,
        rect,
        context,
    } = command
    {
        let client = bridge.client.clone();
        let connection = tauri::async_runtime::spawn_blocking(move || {
            client
                .lock()
                .map_err(|_| player_bridge::error("Client indisponible."))?
                .playback_connection()
        })
        .await
        .map_err(|_| player_bridge::error("Client interrompu."))??;
        bridge
            .player
            .lock()
            .await
            .start(&window, connection, media_id, rect, context)
            .await
    } else {
        bridge.player.lock().await.command(&window, command).await
    }
}
#[tauri::command]
async fn artwork_command(
    app: tauri::AppHandle,
    state: tauri::State<'_, Bridge>,
    target: artwork_bridge::ArtworkTarget,
) -> Result<Value, UiError> {
    let bridge = state.inner().clone();
    let permit = bridge
        .slot
        .clone()
        .try_acquire_owned()
        .map_err(|_| UiError {
            message: "Une opération est déjà en cours.".into(),
            status: None,
            code: "CLIENT_BUSY".into(),
        })?;
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
        .pick_file(move |file| {
            let _ = tx.send(file.and_then(|f| f.as_path().map(|p| p.to_owned())));
        });
    let Some(path) = rx.await.map_err(|_| UiError {
        message: "Sélecteur d'image interrompu.".into(),
        status: None,
        code: "CLIENT_INTERRUPTED".into(),
    })?
    else {
        return Ok(serde_json::json!({"cancelled":true}));
    };
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let (bytes, mime) = artwork_bridge::read_selected(&path)?;
        bridge
            .client
            .lock()
            .map_err(|_| UiError {
                message: "Client indisponible.".into(),
                status: None,
                code: "CLIENT_UNAVAILABLE".into(),
            })
            .and_then(|mut client| artwork_bridge::upload(&mut client, target, bytes, mime))
    })
    .await
    .map_err(|_| UiError {
        message: "Opération interrompue.".into(),
        status: None,
        code: "CLIENT_INTERRUPTED".into(),
    })?
}
#[tauri::command]
async fn admin_command(
    app: tauri::AppHandle,
    state: tauri::State<'_, Bridge>,
    command: admin_bridge::AdminCommand,
) -> Result<Value, UiError> {
    use admin_bridge::{AdminCommand as C, error};
    let bridge = state.inner().clone();
    if let C::Cancel { id } = &command {
        let job = bridge.administration.get(id)?;
        if job
            .lock()
            .map_err(|_| error("Administration indisponible.", "ADMIN_BUSY"))?
            .view
            .stage
            == "applying"
        {
            return Err(error(
                "Import déjà confirmé : attendez son résultat. Une interruption ne garantit pas un retour arrière.",
                "ADMIN_CONFIRMED",
            ));
        }
        admin_bridge::cancel(job);
        return Ok(serde_json::json!({"cancelled":true}));
    }
    let client = bridge.client.clone();
    let connection = tauri::async_runtime::spawn_blocking(move || {
        client
            .lock()
            .map_err(|_| error("Client indisponible.", "ADMIN_BUSY"))?
            .playback_connection()
    })
    .await
    .map_err(|_| error("Client interrompu.", "ADMIN_BUSY"))??;
    let job = match &command {
        C::Import {} => bridge.administration.begin(connection, "import")?,
        C::Export {} => bridge.administration.begin(connection, "export")?,
        C::Status { id } | C::Save { id } | C::Plan { id, .. } | C::Apply { id, .. } => {
            let job = bridge.administration.get(id)?;
            {
                let locked = job
                    .lock()
                    .map_err(|_| error("Administration indisponible.", "ADMIN_BUSY"))?;
                admin_bridge::check(&locked, &connection)?;
            }
            job
        }
        C::Cancel { .. } => unreachable!(),
    };
    match command {
        C::Import {} => {
            let selected = archive_choice(&app, false).await?;
            if let Some(path) = selected {
                let file = match tauri::async_runtime::spawn_blocking(move || {
                    admin_bridge::selected_zip(&path)
                })
                .await
                .map_err(|_| error("Sélection interrompue.", "ADMIN_PICKER"))?
                {
                    Ok(file) => file,
                    Err(err) => {
                        if let Ok(mut j) = job.lock() {
                            j.busy = false;
                        }
                        admin_bridge::cancel(job);
                        return Err(err);
                    }
                };
                let copy = job.clone();
                tauri::async_runtime::spawn(admin_bridge::upload(copy, file));
            } else {
                if let Ok(mut j) = job.lock() {
                    j.busy = false;
                }
                admin_bridge::cancel(job);
                return Ok(serde_json::json!({"cancelled":true}));
            }
        }
        C::Export {} => {
            tauri::async_runtime::spawn(admin_bridge::export(job.clone()));
        }
        C::Save { .. } => {
            {
                let mut j = job
                    .lock()
                    .map_err(|_| error("Administration indisponible.", "ADMIN_BUSY"))?;
                if j.busy || j.view.stage != "ready" {
                    return Err(error("Export non prêt.", "ADMIN_BUSY"));
                }
                j.busy = true;
            }
            let selected = archive_choice(&app, true).await?;
            if let Some(path) = selected {
                let result = tauri::async_runtime::spawn_blocking(move || {
                    admin_bridge::Output::create(&path)
                })
                .await
                .map_err(|_| error("Sélection interrompue.", "ADMIN_PICKER"))?;
                match result {
                    Ok((output, file)) => {
                        if let Ok(mut j) = job.lock() {
                            j.view.stage = "saving";
                        }
                        tauri::async_runtime::spawn(admin_bridge::save(job.clone(), output, file));
                    }
                    Err(err) => {
                        if let Ok(mut j) = job.lock() {
                            j.busy = false;
                        }
                        return Err(err);
                    }
                }
            } else {
                if let Ok(mut j) = job.lock() {
                    j.busy = false;
                }
                return Ok(serde_json::json!({"cancelled":true}));
            }
        }
        C::Plan { mapping, .. } => {
            if mapping.profile_map.is_empty()
                || mapping.profile_map.len() > 1000
                || mapping.root_prefix.len() > 2000
            {
                return Err(error("Correspondances invalides.", "ADMIN_INVALID"));
            }
            {
                let mut j = job
                    .lock()
                    .map_err(|_| error("Administration indisponible.", "ADMIN_BUSY"))?;
                if j.busy || j.preview.is_none() {
                    return Err(error("Aperçu non prêt.", "ADMIN_BUSY"));
                }
                j.busy = true;
                j.plan = None;
                j.view.stage = "planning";
            }
            tauri::async_runtime::spawn(admin_bridge::plan(job.clone(), mapping));
        }
        C::Apply { confirmed, .. } => {
            if !confirmed {
                return Err(error(
                    "Confirmation explicite requise.",
                    "ADMIN_CONFIRMATION",
                ));
            }
            {
                let mut j = job
                    .lock()
                    .map_err(|_| error("Administration indisponible.", "ADMIN_BUSY"))?;
                if j.busy || j.view.stage != "planned" || j.plan.is_none() {
                    return Err(error("Plan non prêt.", "ADMIN_BUSY"));
                }
                j.busy = true;
                j.view.stage = "applying";
            }
            tauri::async_runtime::spawn(admin_bridge::apply(job.clone()));
        }
        C::Status { .. } => {}
        C::Cancel { .. } => unreachable!(),
    }
    let view = job
        .lock()
        .map_err(|_| error("Administration indisponible.", "ADMIN_BUSY"))?
        .view
        .clone();
    serde_json::to_value(view).map_err(|_| error("Réponse invalide.", "ADMIN_INVALID"))
}
async fn archive_choice(
    app: &tauri::AppHandle,
    saving: bool,
) -> Result<Option<std::path::PathBuf>, UiError> {
    #[cfg(debug_assertions)]
    if std::env::var("CHILL_UI_QA_MODE").ok().as_deref() == Some("admin_archive")
        && std::env::var_os("CHILL_UI_QA_URL").is_some()
    {
        let directory = std::env::var_os("CHILL_UI_QA_DATA")
            .ok_or_else(|| admin_bridge::error("Dossier QA requis.", "QA_INVALID"))?;
        return Ok(Some(
            std::path::PathBuf::from(directory).join("native-admin-roundtrip.zip"),
        ));
    }
    let (tx, rx) = tokio::sync::oneshot::channel();
    let dialog = app.dialog().file().add_filter("Archive ZIP", &["zip"]);
    let callback = move |f: Option<tauri_plugin_dialog::FilePath>| {
        let _ = tx.send(f.and_then(|v| v.as_path().map(std::path::PathBuf::from)));
    };
    if saving {
        dialog.set_file_name("Chill-Center.zip").save_file(callback);
    } else {
        dialog.pick_file(callback);
    }
    rx.await
        .map_err(|_| admin_bridge::error("Sélecteur interrompu.", "ADMIN_PICKER"))
}
fn main() {
    let arguments:Vec<_>=std::env::args_os().collect();
    if arguments.len()==2&&arguments[1]=="--verify-bundle"{
        match chill_player::verify_standalone_library(){
            Ok((path,major,minor))=>{println!("BUNDLE_LIBRARY_OK {} API {major}.{minor}",path.display());std::process::exit(0);}
            Err(error)=>{eprintln!("BUNDLE_LIBRARY_FAILED {error}");std::process::exit(1);}
        }
    }
    let builder = tauri::Builder::default().setup(|app| {
        let directory = app.path().app_config_dir()?;
        #[cfg(debug_assertions)]
        let directory = if std::env::var_os("CHILL_UI_QA_URL").is_some() {
            std::env::var_os("CHILL_UI_QA_DATA")
                .ok_or_else(|| std::io::Error::other("QA requires isolated client directory"))?
                .into()
        } else {
            directory
        };
        let client = UiClient::open(&directory).map_err(|e| std::io::Error::other(e.message))?;
        let mut player = PlayerHost::default();
        player.cache_dir = directory.join("subtitle-cache");
        let rendering = player.rendering.clone();
        let assistant = Arc::new(
            ai_bridge::AiHost::open(directory.clone())
                .map_err(|e| std::io::Error::other(e.message))?,
        );
        app.manage(Bridge {
            client: Arc::new(Mutex::new(client)),
            slot: Arc::new(tokio::sync::Semaphore::new(1)),
            player: Arc::new(tokio::sync::Mutex::new(player)),
            closing: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            rendering,
            administration: Arc::new(Default::default()),
            assistant,
        });
        app.handle().plugin(tauri_plugin_dialog::init())?;
        let window = tauri::WebviewWindowBuilder::from_config(app, &app.config().app.windows[0])?
            .on_navigation(|url| {
                (url.scheme() == "tauri" && url.host_str() == Some("localhost"))
                    || (url.scheme() == "http" && url.host_str() == Some("tauri.localhost"))
            })
            .build()?;
        let bridge = app.state::<Bridge>().inner().clone();
        #[cfg(debug_assertions)]
        if matches!(std::env::var("CHILL_UI_QA_MODE").as_deref(),Ok("formats"|"identity_prepare"|"identity_finish")) {
            let probe_window=window.clone();
            tauri::async_runtime::spawn(async move {
                let mut pulse=tokio::time::interval(std::time::Duration::from_secs(5));
                loop {pulse.tick().await;let _=probe_window.eval("window.__TAURI__.core.invoke('qa_result',{success:false,detail:'TRACE probe '+JSON.stringify({busy:playerBusy,stage:playerState.stage,note:document.getElementById('player-note').textContent,hidden:document.hidden})});");}
            });
        }
        let tick_window = window.clone();
        let tick_bridge = bridge.clone();
        tauri::async_runtime::spawn(async move {
            let mut timer = tokio::time::interval(std::time::Duration::from_millis(250));
            timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            while !tick_bridge
                .closing
                .load(std::sync::atomic::Ordering::Acquire)
            {
                timer.tick().await;
                if let Ok(_permit) = tick_bridge.slot.clone().try_acquire_owned() {
                    let _ = tick_bridge.player.lock().await.tick(&tick_window).await;
                }
            }
        });
        let render_window = window.clone();
        let guard_window = window.clone();
        tauri::async_runtime::spawn(async move {
            let mut timer = tokio::time::interval(std::time::Duration::from_millis(250));
            timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                timer.tick().await;
                let _ = native_player::deadline_tick(&guard_window).await;
            }
        });
        let render_bridge = bridge.clone();
        tauri::async_runtime::spawn(async move {
            let mut timer = tokio::time::interval(std::time::Duration::from_millis(16));
            timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                timer.tick().await;
                if render_bridge
                    .closing
                    .load(std::sync::atomic::Ordering::Acquire)
                {
                    break;
                }
                let active = render_bridge
                    .rendering
                    .load(std::sync::atomic::Ordering::Acquire);
                if active {
                    let _ = native_player::render_tick(&render_window).await;
                }
            }
        });
        let closing_window = window.clone();
        let handle = app.handle().clone();
        window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                bridge.administration.cancel_all();
                bridge.assistant.cancel_all();
                api.prevent_close();
                if !bridge
                    .closing
                    .swap(true, std::sync::atomic::Ordering::AcqRel)
                {
                    let bridge = bridge.clone();
                    let w = closing_window.clone();
                    let app = handle.clone();
                    tauri::async_runtime::spawn(async move {
                        // Pause before waiting behind an in-flight network operation.
                        let _ = native_player::close_if_loaded(&w).await;
                        bridge.player.lock().await.stop(&w).await.ok();
                        app.exit(0);
                    });
                }
            }
        });
        Ok(())
    });
    #[cfg(debug_assertions)]
    let builder = builder
        .invoke_handler(tauri::generate_handler![
            ui_command,
            player_command,
            artwork_command,
            admin_command,
            ai_command,
            qa_deadline_probe,
            qa_result
        ])
        .on_page_load(|webview, payload| {
            if payload.event() != tauri::webview::PageLoadEvent::Finished {
                return;
            }
            if let (Ok(address), Ok(root)) = (
                std::env::var("CHILL_UI_QA_URL"),
                std::env::var("CHILL_UI_QA_ROOT"),
            ) {
                let mode = std::env::var("CHILL_UI_QA_MODE").unwrap_or_default();
                let _ = webview.eval(format!(
                    "{}\n{}({});",
                    [include_str!("../../../ui/connected/qa.js"),
                    include_str!("../../../ui/connected/player-qa.js"),
                    include_str!("../../../ui/connected/catalog-qa.js"),
                    include_str!("../../../ui/connected/archive-qa.js"),
                    include_str!("../../../ui/connected/ai-qa.js"),
                    include_str!("../../../ui/connected/deadline-qa.js"),
                    include_str!("../../../ui/connected/formats-qa.js"),
                    include_str!("../../../ui/connected/identity-qa.js")].join("\n"),
                    if mode == "player" || mode == "next_series" || mode.starts_with("transfer_") {
                        "window.chillPlayerQa"
                    } else {
                        if mode.starts_with("identity_") {
                            "window.chillIdentityRoundtripQa"
                        } else if mode == "formats" {
                            "window.chillFormatsQa"
                        } else if mode == "deadline" {
                            "window.chillDeadlineQa"
                        } else if mode == "ai" {
                            "window.chillAiQa"
                        } else if mode == "admin_archive" {
                            "window.chillArchiveQa"
                        } else if mode == "catalog" {
                            "window.chillCatalogQa"
                        } else {
                            "window.chillQaSmoke"
                        }
                    },
                    serde_json::json!({"address":address,"root":root,"mode":mode,"cli":std::env::var("CHILL_UI_QA_CLI").ok()})
                ));
            }
        });
    #[cfg(not(debug_assertions))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        ui_command,
        player_command,
        artwork_command,
        admin_command,
        ai_command
    ]);
    builder
        .run(tauri::generate_context!())
        .expect("Chill Center startup");
}
#[cfg(debug_assertions)]
#[tauri::command]
async fn qa_deadline_probe(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, Bridge>,
) -> Result<Value, UiError> {
    if std::env::var("CHILL_UI_QA_MODE").as_deref() != Ok("deadline")
        || !std::env::var("CHILL_UI_QA_URL").is_ok_and(|v| v.starts_with("https://127.0.0.1:"))
    {
        return Err(ai_bridge::error("QA indisponible.", "QA_DISABLED"));
    }
    let bridge = state.inner().clone();
    let _permit = bridge
        .slot
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| ai_bridge::error("QA interrompue.", "QA_FAILED"))?;
    let _host = bridge.player.lock().await;
    let rendering_before = bridge
        .rendering
        .swap(false, std::sync::atomic::Ordering::AcqRel);
    let result=async {
        native_player::action(&window,native_player::NativeAction::LeaseDeadline{remaining_ms:500}).await.map_err(|_|ai_bridge::error("QA lecteur indisponible.","QA_FAILED"))?;
        let initially_paused=native_player::guard_paused(&window).await.map_err(|_|ai_bridge::error("QA lecteur indisponible.","QA_FAILED"))?;
        tokio::time::sleep(std::time::Duration::from_millis(900)).await;
        let paused=native_player::guard_paused(&window).await.map_err(|_|ai_bridge::error("QA lecteur indisponible.","QA_FAILED"))?;
        Ok(serde_json::json!({"initially_paused":initially_paused,"paused":paused,"rendering":bridge.rendering.load(std::sync::atomic::Ordering::Acquire),"ui_slot_held":bridge.slot.available_permits()==0,"host_mutex_held":bridge.player.try_lock().is_err()}))
    }.await;
    bridge
        .rendering
        .store(rendering_before, std::sync::atomic::Ordering::Release);
    result
}
#[cfg(debug_assertions)]
#[tauri::command]
fn qa_result(app: tauri::AppHandle, success: bool, detail: String) {
    if std::env::var_os("CHILL_UI_QA_URL").is_some() {
        if detail.starts_with("TRACE ") {
            println!("UI_QA_TRACE {}", detail.chars().take(300).collect::<String>());
            return;
        }
        println!(
            "UI_QA_RESULT {} {}",
            success,
            detail.chars().take(300).collect::<String>()
        );
        app.exit(if success { 0 } else { 1 });
    }
}
