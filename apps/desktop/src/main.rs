use chill_client::ui::{UiClient, UiCommand, UiError};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tauri::Manager;

#[derive(Clone)]
struct Bridge {
    client: Arc<Mutex<UiClient>>,
    slot: Arc<tokio::sync::Semaphore>,
}
#[tauri::command]
async fn ui_command(state: tauri::State<'_, Bridge>, command: UiCommand) -> Result<Value, UiError> {
    let bridge = state.inner().clone();
    let permit = bridge
        .slot
        .clone()
        .try_acquire_owned()
        .map_err(|_| UiError {
            message: "Une opération est déjà en cours.".into(),
            status: None,
        })?;
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        bridge
            .client
            .lock()
            .map_err(|_| UiError {
                message: "Client indisponible.".into(),
                status: None,
            })?
            .dispatch(command)
    })
    .await
    .map_err(|_| UiError {
        message: "Opération interrompue.".into(),
        status: None,
    })?
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
        app.manage(Bridge {
            client: Arc::new(Mutex::new(client)),
            slot: Arc::new(tokio::sync::Semaphore::new(1)),
        });
        tauri::WebviewWindowBuilder::from_config(app, &app.config().app.windows[0])?
            .on_navigation(|url| {
                (url.scheme() == "tauri" && url.host_str() == Some("localhost"))
                    || (url.scheme() == "http" && url.host_str() == Some("tauri.localhost"))
            })
            .build()?;
        Ok(())
    });
    #[cfg(debug_assertions)]
    let builder = builder
        .invoke_handler(tauri::generate_handler![ui_command, qa_result])
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
                    "{}\nwindow.chillQaSmoke({});",
                    include_str!("../../../ui/connected/qa.js"),
                    serde_json::json!({"address":address,"root":root,"mode":mode})
                ));
            }
        });
    #[cfg(not(debug_assertions))]
    let builder = builder.invoke_handler(tauri::generate_handler![ui_command]);
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
