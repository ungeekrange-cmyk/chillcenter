//! CU88-TEST005: adversarial, public-only boundaries around updater inputs.

use chill_updates::{
    ArchiveKind, Component, ExtractionLimits, IntentKind, OperationStatus, RuntimeCompatibility,
    StateStore, VerificationContext, extract_archive, parse_and_verify_manifest,
    verify_staged_artifact,
};
use semver::Version;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use zip::write::SimpleFileOptions;

fn vectors() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors")
}

#[test]
fn signed_mac_web_and_theme_fixtures_reach_the_production_verifier() {
    let directory = vectors();
    let public = fs::read_to_string(directory.join("hostile.pub")).unwrap();
    let key_id = public
        .lines()
        .next()
        .unwrap()
        .split_ascii_whitespace()
        .last()
        .unwrap();
    for (name, component, target) in [
        ("mac", Component::Mac, "darwin-aarch64"),
        ("web", Component::Web, "linux-aarch64"),
        ("themes", Component::Themes, "any"),
    ] {
        let bytes = fs::read(directory.join(format!("hostile-{name}-valid.json"))).unwrap();
        let signature =
            fs::read_to_string(directory.join(format!("hostile-{name}-valid.json.minisig")))
                .unwrap();
        let installed = Version::new(0, 6, 0);
        let context = VerificationContext {
            now: OffsetDateTime::parse("2026-10-19T12:00:00Z", &Rfc3339).unwrap(),
            expected_key_id: key_id,
            component,
            target,
            installed_version: &installed,
            highest_sequence: 1,
            runtime: RuntimeCompatibility {
                api_version: None,
                server_version: None,
                database_schema: None,
                renderer_version: Some(1),
                theme_format: Some(1),
            },
        };
        let verified = parse_and_verify_manifest(&bytes, &signature, &public, &context).unwrap();
        assert_eq!(verified.manifest.component, component);
        assert_eq!(verified.selected_asset.target, target);
    }
}

fn write_zip(path: &Path, files: &[(&str, &[u8], u32)], deflated: bool) {
    let mut zip = zip::ZipWriter::new(File::create(path).unwrap());
    for (name, bytes, permissions) in files {
        let mut options = SimpleFileOptions::default().unix_permissions(*permissions);
        if deflated {
            options = options.compression_method(zip::CompressionMethod::Deflated);
        }
        zip.start_file(*name, options).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap();
}

fn extract_zip(
    path: &Path,
    destination: &Path,
    limits: ExtractionLimits,
) -> chill_updates::Result<()> {
    extract_archive(path, destination, ArchiveKind::Zip, &limits)
}

fn mark_first_zip_entry_as_symlink(path: &Path) {
    let mut bytes = fs::read(path).unwrap();
    let central = bytes
        .windows(4)
        .position(|window| window == b"PK\x01\x02")
        .unwrap();
    bytes[central + 4] = 20; // ZIP version made by
    bytes[central + 5] = 3; // Unix creator system
    bytes[central + 38..central + 42].copy_from_slice(&(0o120777_u32 << 16).to_le_bytes());
    fs::write(path, bytes).unwrap();
}

fn make_duplicate_zip_entry(path: &Path) {
    // ZIP permits both spellings; `enclosed_name` canonicalizes them to the
    // same safe output path, where the extractor must still reject a merge.
    write_zip(
        path,
        &[
            ("same", b"one", 0o100644),
            ("same/../same", b"two", 0o100644),
        ],
        false,
    );
}

#[test]
fn zip_special_duplicate_and_limit_inputs_are_refused_without_staging_residue() {
    let temp = tempfile::tempdir().unwrap();
    let normal = temp.path().join("normal.zip");
    write_zip(&normal, &[("folder/payload", b"ok", 0o100644)], false);
    extract_zip(
        &normal,
        &temp.path().join("normal"),
        ExtractionLimits::default(),
    )
    .unwrap();

    let special = temp.path().join("special.zip");
    let special_destination = temp.path().join("special");
    write_zip(&special, &[("link", b"/etc/passwd", 0o100644)], false);
    mark_first_zip_entry_as_symlink(&special);
    assert!(extract_zip(&special, &special_destination, ExtractionLimits::default()).is_err());
    assert!(!special_destination.exists());

    let duplicate = temp.path().join("duplicate.zip");
    let duplicate_destination = temp.path().join("duplicate");
    make_duplicate_zip_entry(&duplicate);
    assert!(
        extract_zip(
            &duplicate,
            &duplicate_destination,
            ExtractionLimits::default()
        )
        .is_err()
    );
    assert!(!duplicate_destination.exists());

    let cases: Vec<(&str, Vec<(&str, &[u8], u32)>, bool, ExtractionLimits)> = vec![
        (
            "entry-limit",
            vec![("one", b"1", 0o100644), ("two", b"2", 0o100644)],
            false,
            ExtractionLimits {
                max_entries: 1,
                ..ExtractionLimits::default()
            },
        ),
        (
            "file-limit",
            vec![("large", b"1234", 0o100644)],
            false,
            ExtractionLimits {
                max_file_bytes: 3,
                ..ExtractionLimits::default()
            },
        ),
        (
            "total-limit",
            vec![("one", b"12", 0o100644), ("two", b"34", 0o100644)],
            false,
            ExtractionLimits {
                max_total_bytes: 3,
                ..ExtractionLimits::default()
            },
        ),
        (
            "ratio-limit",
            vec![("compressed", &[0; 4096], 0o100644)],
            true,
            ExtractionLimits {
                max_compression_ratio: 2,
                ..ExtractionLimits::default()
            },
        ),
    ];
    for (name, files, deflated, limits) in cases {
        let archive = temp.path().join(format!("{name}.zip"));
        let destination = temp.path().join(name);
        write_zip(&archive, &files, deflated);
        assert!(
            extract_zip(&archive, &destination, limits).is_err(),
            "{name}"
        );
        assert!(
            !destination.exists(),
            "failed extraction leaves no stage: {name}"
        );
    }
}

#[test]
fn raw_artifact_hash_and_byte_bounds_are_exact() {
    let temp = tempfile::tempdir().unwrap();
    let artifact = temp.path().join("server");
    fs::write(&artifact, b"payload").unwrap();
    let digest = hex::encode(Sha256::digest(b"payload"));
    verify_staged_artifact(&artifact, 7, &digest).unwrap();
    assert!(verify_staged_artifact(&artifact, 6, &digest).is_err());
    assert!(verify_staged_artifact(&artifact, 7, &"0".repeat(64)).is_err());
}

#[test]
fn corrupt_state_is_refused_and_completed_history_is_trimmed() {
    let temp = tempfile::tempdir().unwrap();
    let store = StateStore::open(temp.path()).unwrap();
    fs::write(temp.path().join("state.json"), b"{ definitely not json").unwrap();
    assert!(store.read().is_err());
    fs::remove_file(temp.path().join("state.json")).unwrap();

    let components = [
        Component::Mac,
        Component::Server,
        Component::Web,
        Component::Themes,
    ];
    let epoch = OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap();
    for index in 0..129_i64 {
        let now = epoch + time::Duration::seconds(index * 301);
        let operation = store
            .admit_intent(
                components[index as usize % components.len()],
                IntentKind::Check,
                now,
            )
            .unwrap();
        store
            .transition(
                operation.id,
                OperationStatus::Pending,
                OperationStatus::Succeeded,
                None,
                now,
            )
            .unwrap();
    }
    assert_eq!(store.read().unwrap().operations.len(), 128);
}
