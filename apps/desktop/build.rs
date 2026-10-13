fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&["ui_command", "player_command", "qa_result"]),
    ))
    .expect("Tauri configuration");
}
