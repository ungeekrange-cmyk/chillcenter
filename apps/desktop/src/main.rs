use chill_client::ui::{UiClient, UiCommand, UiError};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tauri::Manager;
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
fn main() {
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
        app.manage(Bridge {
            client: Arc::new(Mutex::new(client)),
            slot: Arc::new(tokio::sync::Semaphore::new(1)),
            player: Arc::new(tokio::sync::Mutex::new(player)),
            closing: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            rendering,
        });
        let window = tauri::WebviewWindowBuilder::from_config(app, &app.config().app.windows[0])?
            .on_navigation(|url| {
                (url.scheme() == "tauri" && url.host_str() == Some("localhost"))
                    || (url.scheme() == "http" && url.host_str() == Some("tauri.localhost"))
            })
            .build()?;
        let bridge = app.state::<Bridge>().inner().clone();
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
                api.prevent_close();
                if !bridge
                    .closing
                    .swap(true, std::sync::atomic::Ordering::AcqRel)
                {
                    let bridge = bridge.clone();
                    let w = closing_window.clone();
                    let app = handle.clone();
                    tauri::async_runtime::spawn(async move {
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
                    "{}\n{}\n{}\n{}({});",
                    include_str!("../../../ui/connected/qa.js"),
                    include_str!("../../../ui/connected/player-qa.js"),
                    include_str!("../../../ui/connected/catalog-qa.js"),
                    if mode == "player" || mode == "next_series" || mode.starts_with("transfer_") {
                        "window.chillPlayerQa"
                    } else {
                        if mode == "catalog" {
                            "window.chillCatalogQa"
                        } else {
                            "window.chillQaSmoke"
                        }
                    },
                    serde_json::json!({"address":address,"root":root,"mode":mode})
                ));
            }
        });
    #[cfg(not(debug_assertions))]
    let builder = builder.invoke_handler(tauri::generate_handler![ui_command, player_command]);
    builder
        .run(tauri::generate_context!())
        .expect("Chill Center startup");
}
#[cfg(debug_assertions)]
#[tauri::command]
fn qa_result(app: tauri::AppHandle, success: bool, detail: String) {
    if std::env::var_os("CHILL_UI_QA_URL").is_some() {
        println!(
            "UI_QA_RESULT {} {}",
            success,
            detail.chars().take(300).collect::<String>()
        );
        app.exit(if success { 0 } else { 1 });
    }
}
