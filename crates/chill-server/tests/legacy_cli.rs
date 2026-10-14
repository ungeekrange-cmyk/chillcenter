use chill_catalog::{database::Database, model::Media};
use std::{fs, path::Path, process::Command};
fn invoke(source: &Path, cache: &Path, output: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_chill-server"))
        .arg("--export-cinefolio")
        .arg("--source-db")
        .arg(source)
        .arg("--image-cache")
        .arg(cache)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}
#[test]
fn actual_cli_exports_without_starting_server_or_overwriting_destination() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let source = root.join("cinefolio.sqlite");
    let cache = root.join("images");
    fs::create_dir(&cache).unwrap();
    let db = Database::open(&source).unwrap();
    let mut media:Media=serde_json::from_value(serde_json::json!({"id":"Film.mkv","path":"/NAS/Film.mkv","title":"Film","category":"film","year":2020,"series":null,"season":null,"episode":null,"genres":[],"overview":"Fixture","poster":null,"saga":null,"watched":true,"available":true,"added_at":12,"tmdb_id":null})).unwrap();
    media.rating = Some(3);
    db.put(&media).unwrap();
    drop(db);
    let before = fs::read(&source).unwrap();
    let output = root.join("portable.zip");
    let result = invoke(&source, &cache, &output);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output.exists());
    assert_eq!(before, fs::read(&source).unwrap());
    let archive = fs::read(&output).unwrap();
    assert!(!invoke(&source, &cache, &output).status.success());
    assert_eq!(archive, fs::read(&output).unwrap());
    assert!(!root.join("tls").exists());
    for entry in fs::read_dir(&root).unwrap() {
        let name = entry.unwrap().file_name();
        assert!(
            [
                "cinefolio.sqlite",
                "cinefolio.sqlite-wal",
                "cinefolio.sqlite-shm",
                "images",
                "portable.zip"
            ]
            .iter()
            .any(|allowed| name == *allowed),
            "Only SQLite's read-lock sidecars may remain beside source/output/cache; no scratch workspace"
        );
    }
}
#[test]
fn actual_cli_rejects_future_source_schema_before_output_or_source_mutation() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let source = root.join("future.sqlite");
    let db = rusqlite::Connection::open(&source).unwrap();
    db.pragma_update(None, "user_version", 99).unwrap();
    drop(db);
    let cache = root.join("cache");
    fs::create_dir(&cache).unwrap();
    let before = fs::read(&source).unwrap();
    let output = root.join("out.zip");
    assert!(!invoke(&source, &cache, &output).status.success());
    assert_eq!(before, fs::read(&source).unwrap());
    assert!(!output.exists());
}
