use crate::archive::{ArchiveKind, ExtractionLimits, extract_archive};
use crate::manifest::{AssetFormat, Component, VerifiedManifest};
use crate::{Result, UpdateError};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct InstallLayout {
    pub component_root: PathBuf,
    pub backup_root: PathBuf,
}

pub struct ApplyRequest<'a> {
    pub operation_id: Uuid,
    pub verified: &'a VerifiedManifest,
    pub artifact: &'a Path,
    pub layout: &'a InstallLayout,
    pub limits: &'a ExtractionLimits,
    pub allow_first_install: bool,
    pub force_interrupt: bool,
}

#[derive(Clone, Debug)]
pub struct ApplyResult {
    pub installed: PathBuf,
    pub previous: Option<PathBuf>,
    pub backup_id: String,
}

/// Integration hooks are implemented by the privileged Linux helper/server
/// glue. Implementations must use fixed root-owned configuration and exact
/// argv; no shell or service-owned executable is accepted.
pub trait InstallHooks {
    fn is_idle(&mut self, component: Component) -> Result<bool>;
    fn begin_quiescence(&mut self, component: Component, force_interrupt: bool) -> Result<()>;
    fn backup(&mut self, component: Component, backup_root: &Path) -> Result<String>;
    fn activate(&mut self, component: Component, installed_root: &Path) -> Result<()>;
    fn health_check(&mut self, component: Component, installed_root: &Path) -> Result<()>;
    fn end_quiescence(&mut self, component: Component) -> Result<()>;
}

pub fn apply_staged(
    request: &ApplyRequest<'_>,
    hooks: &mut dyn InstallHooks,
) -> Result<ApplyResult> {
    validate_layout(request.layout)?;
    verify_artifact(
        request.artifact,
        request.verified.selected_asset.byte_length,
        &request.verified.selected_asset.sha256,
    )?;
    if request.verified.manifest.component == Component::Server
        && !matches!(request.verified.selected_asset.format, AssetFormat::Raw)
    {
        return Err(UpdateError::Security(
            "server payload must be one reviewed raw binary".into(),
        ));
    }
    if !hooks.is_idle(request.verified.manifest.component)? && !request.force_interrupt {
        return Err(UpdateError::Busy(
            "playback or conflicting work is active".into(),
        ));
    }
    hooks.begin_quiescence(request.verified.manifest.component, request.force_interrupt)?;
    let outcome = apply_while_quiesced(request, hooks);
    let resumed = hooks.end_quiescence(request.verified.manifest.component);
    match (outcome, resumed) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
    }
}

fn apply_while_quiesced(
    request: &ApplyRequest<'_>,
    hooks: &mut dyn InstallHooks,
) -> Result<ApplyResult> {
    if !hooks.is_idle(request.verified.manifest.component)? && !request.force_interrupt {
        return Err(UpdateError::Busy(
            "work began before the final commit window".into(),
        ));
    }
    let releases = request.layout.component_root.join("releases");
    fs::create_dir_all(&releases)?;
    reject_symlink(&releases)?;
    let leaf = format!(
        "{}-{}",
        request.verified.manifest.version, request.verified.manifest.release_sequence
    );
    let final_root = releases.join(&leaf);
    if fs::symlink_metadata(&final_root).is_ok() {
        return Err(UpdateError::Security(
            "release directory already exists".into(),
        ));
    }
    let stage = releases.join(format!(".stage-{}", request.operation_id));
    let kind = match request.verified.selected_asset.format {
        AssetFormat::Raw => ArchiveKind::Raw,
        AssetFormat::Zip => ArchiveKind::Zip,
        AssetFormat::Tar => ArchiveKind::Tar,
    };
    extract_archive(request.artifact, &stage, kind, request.limits)?;
    if request.verified.manifest.component == Component::Themes {
        validate_theme_tree(&stage, request.limits)?;
    }
    #[cfg(unix)]
    if request.verified.manifest.component == Component::Server {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(stage.join("payload"), fs::Permissions::from_mode(0o755))?;
    }
    fs::rename(&stage, &final_root)?;
    File::open(&releases)?.sync_all()?;

    let current = request.layout.component_root.join("current");
    let previous_target = match fs::symlink_metadata(&current) {
        Ok(metadata) => {
            if !metadata.file_type().is_symlink() {
                return Err(UpdateError::Security(
                    "current slot is not a symlink".into(),
                ));
            }
            let target = fs::read_link(&current)?;
            validate_slot_target(&target)?;
            Some(target)
        }
        Err(error)
            if error.kind() == std::io::ErrorKind::NotFound && request.allow_first_install =>
        {
            None
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(UpdateError::Security(
                "installed current slot is absent".into(),
            ));
        }
        Err(error) => return Err(error.into()),
    };

    let backup_id = hooks.backup(
        request.verified.manifest.component,
        &request.layout.backup_root,
    )?;
    if backup_id.is_empty()
        || backup_id.len() > 128
        || !backup_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
    {
        return Err(UpdateError::Security(
            "backup hook returned an unsafe identifier".into(),
        ));
    }
    let new_target = PathBuf::from("releases").join(&leaf);
    let new_link = request
        .layout
        .component_root
        .join(format!(".current-{}", request.operation_id));
    create_symlink(&new_target, &new_link)?;
    if let Some(old_target) = &previous_target {
        let previous_link = request
            .layout
            .component_root
            .join(format!("previous-{}", request.operation_id));
        create_symlink(old_target, &previous_link)?;
    }
    fs::rename(&new_link, &current)?;
    File::open(&request.layout.component_root)?.sync_all()?;

    let activation = hooks
        .activate(request.verified.manifest.component, &final_root)
        .and_then(|_| hooks.health_check(request.verified.manifest.component, &final_root));
    if let Err(failure) = activation {
        if let Some(old_target) = &previous_target {
            let rollback_link = request
                .layout
                .component_root
                .join(format!(".rollback-{}", request.operation_id));
            create_symlink(old_target, &rollback_link)?;
            fs::rename(&rollback_link, &current)?;
            File::open(&request.layout.component_root)?.sync_all()?;
            let old_root = request.layout.component_root.join(old_target);
            hooks.activate(request.verified.manifest.component, &old_root)?;
            hooks.health_check(request.verified.manifest.component, &old_root)?;
        } else {
            fs::remove_file(&current)?;
            File::open(&request.layout.component_root)?.sync_all()?;
        }
        return Err(UpdateError::Security(format!(
            "health check failed; previous slot restored: {failure}"
        )));
    }
    Ok(ApplyResult {
        installed: final_root,
        previous: previous_target.map(|target| request.layout.component_root.join(target)),
        backup_id,
    })
}

fn validate_theme_tree(root: &Path, limits: &ExtractionLimits) -> Result<()> {
    let mut stack = vec![root.to_path_buf()];
    let mut manifest_count = 0usize;
    let mut entries = 0usize;
    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            entries += 1;
            if entries > limits.max_entries {
                return Err(UpdateError::Security(
                    "theme tree has too many entries".into(),
                ));
            }
            let metadata = entry.file_type()?;
            if metadata.is_symlink() {
                return Err(UpdateError::Security("theme symlink refused".into()));
            }
            if metadata.is_dir() {
                stack.push(entry.path());
                continue;
            }
            if !metadata.is_file() {
                return Err(UpdateError::Security("theme special file refused".into()));
            }
            let relative = entry
                .path()
                .strip_prefix(root)
                .map_err(|_| UpdateError::Security("theme path escaped root".into()))?
                .to_path_buf();
            if relative == Path::new("manifest.json") {
                manifest_count += 1;
                continue;
            }
            let extension = relative
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if !matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp" | "gif") {
                return Err(UpdateError::Security(
                    "theme contains executable or unsupported asset type".into(),
                ));
            }
        }
    }
    if manifest_count != 1 {
        return Err(UpdateError::Security(
            "theme requires one root manifest.json".into(),
        ));
    }
    Ok(())
}

fn verify_artifact(path: &Path, expected_length: u64, expected_sha256: &str) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() != expected_length
    {
        return Err(UpdateError::Security(
            "staged artifact type/length mismatch".into(),
        ));
    }
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    if !hex::encode(hasher.finalize()).eq_ignore_ascii_case(expected_sha256) {
        return Err(UpdateError::Security(
            "staged artifact hash mismatch".into(),
        ));
    }
    Ok(())
}

fn validate_layout(layout: &InstallLayout) -> Result<()> {
    for root in [&layout.component_root, &layout.backup_root] {
        if !root.is_absolute() {
            return Err(UpdateError::Security(
                "install roots must be absolute".into(),
            ));
        }
        fs::create_dir_all(root)?;
        reject_symlink(root)?;
    }
    if layout.component_root.starts_with(&layout.backup_root)
        || layout.backup_root.starts_with(&layout.component_root)
    {
        return Err(UpdateError::Security(
            "install and backup roots overlap".into(),
        ));
    }
    Ok(())
}

fn validate_slot_target(target: &Path) -> Result<()> {
    let mut components = target.components();
    if components.next() != Some(std::path::Component::Normal("releases".as_ref()))
        || components.next().is_none()
        || components.next().is_some()
    {
        return Err(UpdateError::Security(
            "slot target is outside releases".into(),
        ));
    }
    Ok(())
}

fn reject_symlink(path: &Path) -> Result<()> {
    if fs::symlink_metadata(path)?.file_type().is_symlink() {
        return Err(UpdateError::Security("trusted root is a symlink".into()));
    }
    Ok(())
}

#[cfg(unix)]
fn create_symlink(target: &Path, link: &Path) -> Result<()> {
    std::os::unix::fs::symlink(target, link)?;
    Ok(())
}

#[cfg(not(unix))]
fn create_symlink(_target: &Path, _link: &Path) -> Result<()> {
    Err(UpdateError::Invalid(
        "Linux installer is unavailable on this platform".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{Asset, Compatibility, SignedManifest};
    use semver::Version;
    use std::io::Write;

    struct Hooks {
        healthy: bool,
        activations: Vec<PathBuf>,
    }
    impl InstallHooks for Hooks {
        fn is_idle(&mut self, _: Component) -> Result<bool> {
            Ok(true)
        }
        fn begin_quiescence(&mut self, _: Component, _: bool) -> Result<()> {
            Ok(())
        }
        fn backup(&mut self, _: Component, _: &Path) -> Result<String> {
            Ok("synthetic-backup".into())
        }
        fn activate(&mut self, _: Component, root: &Path) -> Result<()> {
            self.activations.push(root.to_path_buf());
            Ok(())
        }
        fn health_check(&mut self, _: Component, _: &Path) -> Result<()> {
            if self.healthy {
                Ok(())
            } else {
                self.healthy = true;
                Err(UpdateError::Invalid("synthetic health failure".into()))
            }
        }
        fn end_quiescence(&mut self, _: Component) -> Result<()> {
            Ok(())
        }
    }

    fn verified(bytes: &[u8], version: &str, sequence: u64) -> VerifiedManifest {
        let hash = hex::encode(Sha256::digest(bytes));
        let asset = Asset { target: "linux-aarch64".into(), url: "https://github.com/ungeekrange-cmyk/chillcenter/releases/download/server-v0.7.0/server".into(), byte_length: bytes.len() as u64, sha256: hash, format: AssetFormat::Raw, tauri_signature: None };
        VerifiedManifest {
            manifest: SignedManifest {
                schema_version: 1,
                component: Component::Server,
                version: Version::parse(version).unwrap(),
                release_sequence: sequence,
                release_tag: format!("server-v{version}"),
                source_commit: "a".repeat(40),
                issued_at: "2026-10-19T00:00:00Z".into(),
                expires_at: "2026-10-20T00:00:00Z".into(),
                key_id: "test-only".into(),
                compatibility: Compatibility {
                    os_arch: vec!["linux-aarch64".into()],
                    api: None,
                    required_server: None,
                    database: None,
                    renderer: None,
                },
                assets: vec![asset.clone()],
            },
            selected_asset: asset,
        }
    }

    #[test]
    fn rootless_simulation_rolls_back_failed_health_and_retains_new_release() {
        let temp = tempfile::tempdir().unwrap();
        let component_root = temp.path().join("component");
        let backup_root = temp.path().join("backups");
        fs::create_dir_all(component_root.join("releases/0.6.0-1")).unwrap();
        fs::create_dir_all(&backup_root).unwrap();
        create_symlink(
            Path::new("releases/0.6.0-1"),
            &component_root.join("current"),
        )
        .unwrap();
        let artifact = temp.path().join("server");
        File::create(&artifact)
            .unwrap()
            .write_all(b"new-server")
            .unwrap();
        let verified = verified(b"new-server", "0.7.0", 2);
        let layout = InstallLayout {
            component_root: component_root.clone(),
            backup_root,
        };
        let request = ApplyRequest {
            operation_id: Uuid::new_v4(),
            verified: &verified,
            artifact: &artifact,
            layout: &layout,
            limits: &ExtractionLimits::default(),
            allow_first_install: false,
            force_interrupt: false,
        };
        let mut hooks = Hooks {
            healthy: false,
            activations: Vec::new(),
        };
        assert!(apply_staged(&request, &mut hooks).is_err());
        assert_eq!(
            fs::read_link(component_root.join("current")).unwrap(),
            PathBuf::from("releases/0.6.0-1")
        );
        assert!(component_root.join("releases/0.7.0-2/payload").exists());
        assert_eq!(hooks.activations.len(), 2);
    }
}
