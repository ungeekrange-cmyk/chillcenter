fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let commit = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_owned())
        .filter(|value| value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=CHILL_SOURCE_COMMIT={commit}");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "ui_command",
            "player_command",
            "artwork_command",
            "admin_command",
            "ai_command",
            "update_command",
            "theme_command",
            "qa_result",
            "qa_deadline_probe",
        ]),
    ))
    .expect("Tauri configuration");
}
