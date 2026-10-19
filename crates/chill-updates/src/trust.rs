use crate::manifest::Component;
use crate::{Result, UpdateError};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PendingAcceptance {
    pub sequence: u64,
    pub manifest_sha256: String,
    pub version: Version,
    pub accepted_unix: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ComponentTrust {
    highest_sequence: u64,
    pending: Option<PendingAcceptance>,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SequenceState {
    schema_version: u32,
    components: BTreeMap<Component, ComponentTrust>,
}

/// Root-owned monotonic ledger, deliberately separate from service-owned
/// policy/intent state so the unprivileged server cannot enable replay.
pub struct AntiRollbackStore {
    root: PathBuf,
    state: PathBuf,
    lock: PathBuf,
}

impl AntiRollbackStore {
    pub fn open_root(root: impl AsRef<Path>) -> Result<Self> {
        #[cfg(not(unix))]
        return Err(UpdateError::Security(
            "root trust store is Linux-only".into(),
        ));
        #[cfg(unix)]
        {
            let root = root.as_ref().to_path_buf();
            validate_owned_path(&root, 0, true)?;
            Ok(Self {
                state: root.join("sequences.json"),
                lock: root.join("sequences.lock"),
                root,
            })
        }
    }

    pub fn highest(&self, component: Component) -> Result<u64> {
        self.with_lock(|state| {
            Ok((
                state
                    .components
                    .get(&component)
                    .map_or(0, |entry| entry.highest_sequence),
                false,
            ))
        })
    }

    pub fn pending(&self, component: Component) -> Result<Option<PendingAcceptance>> {
        self.with_lock(|state| {
            Ok((
                state
                    .components
                    .get(&component)
                    .and_then(|entry| entry.pending.clone()),
                false,
            ))
        })
    }

    /// An exact pending manifest may be retried at its accepted sequence. No
    /// other manifest receives this exception.
    pub fn verification_floor(
        &self,
        component: Component,
        manifest_sha256: &str,
    ) -> Result<(u64, Option<PendingAcceptance>)> {
        validate_digest(manifest_sha256)?;
        self.with_lock(|state| {
            let Some(entry) = state.components.get(&component) else {
                return Ok(((0, None), false));
            };
            if let Some(pending) = &entry.pending
                && pending.manifest_sha256 == manifest_sha256
            {
                return Ok((
                    (pending.sequence.saturating_sub(1), Some(pending.clone())),
                    false,
                ));
            }
            Ok(((entry.highest_sequence, None), false))
        })
    }

    /// Durably accepts a candidate before its slot can become executable.
    /// Failure and rollback deliberately retain this acceptance.
    pub fn accept_pending(
        &self,
        component: Component,
        sequence: u64,
        manifest_sha256: &str,
        version: &Version,
    ) -> Result<()> {
        validate_digest(manifest_sha256)?;
        if sequence == 0 {
            return Err(UpdateError::Security(
                "zero release sequence refused".into(),
            ));
        }
        self.with_lock(|state| {
            let entry = state.components.entry(component).or_insert(ComponentTrust {
                highest_sequence: 0,
                pending: None,
            });
            let candidate = PendingAcceptance {
                sequence,
                manifest_sha256: manifest_sha256.to_owned(),
                version: version.clone(),
                accepted_unix: time::OffsetDateTime::now_utc().unix_timestamp(),
            };
            if entry.pending.as_ref().is_some_and(|pending| {
                pending.sequence == sequence
                    && pending.manifest_sha256 == manifest_sha256
                    && &pending.version == version
            }) && entry.highest_sequence == sequence
            {
                return Ok(((), false));
            }
            if entry.pending.is_some() {
                return Err(UpdateError::Security(
                    "a different accepted update requires recovery".into(),
                ));
            }
            if sequence <= entry.highest_sequence {
                return Err(UpdateError::Security(
                    "root anti-rollback sequence did not advance".into(),
                ));
            }
            entry.highest_sequence = sequence;
            entry.pending = Some(candidate);
            Ok(((), true))
        })
    }

    pub fn commit_pending(
        &self,
        component: Component,
        sequence: u64,
        manifest_sha256: &str,
    ) -> Result<()> {
        validate_digest(manifest_sha256)?;
        self.with_lock(|state| {
            let entry = state
                .components
                .get_mut(&component)
                .ok_or_else(|| UpdateError::Security("accepted update is absent".into()))?;
            let matches = entry.pending.as_ref().is_some_and(|pending| {
                pending.sequence == sequence && pending.manifest_sha256 == manifest_sha256
            });
            if !matches {
                return Err(UpdateError::Security(
                    "accepted update identity differs at commit".into(),
                ));
            }
            entry.pending = None;
            Ok(((), true))
        })
    }

    fn with_lock<T>(
        &self,
        operation: impl FnOnce(&mut SequenceState) -> Result<(T, bool)>,
    ) -> Result<T> {
        let lock = secure_open()
            .create(true)
            .read(true)
            .write(true)
            .open(&self.lock)?;
        validate_owned_path(&self.lock, 0, false)?;
        if !lock.metadata()?.is_file() || lock.metadata()?.len() > 4096 {
            return Err(UpdateError::Security(
                "anti-rollback lock file is invalid".into(),
            ));
        }
        lock.lock()?;
        let mut state = match secure_open().read(true).open(&self.state) {
            Ok(file) => {
                validate_root_owned_file(&self.state, 64 * 1024)?;
                let mut bytes = Vec::new();
                file.take(64 * 1024 + 1).read_to_end(&mut bytes)?;
                crate::parse_strict_json(&bytes)?
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => SequenceState {
                schema_version: 2,
                components: BTreeMap::new(),
            },
            Err(error) => return Err(error.into()),
        };
        validate_state(&state)?;
        let (result, changed) = operation(&mut state)?;
        if changed {
            state.schema_version = 2;
            validate_state(&state)?;
            let bytes = serde_json::to_vec_pretty(&state)?;
            let temporary = self.root.join(format!(".sequences-{}.tmp", Uuid::new_v4()));
            let mut file = secure_open()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            fs::rename(&temporary, &self.state)?;
            File::open(&self.root)?.sync_all()?;
        }
        lock.unlock()?;
        Ok(result)
    }
}

fn validate_state(state: &SequenceState) -> Result<()> {
    if state.schema_version != 2 || state.components.len() > 4 {
        return Err(UpdateError::Security("invalid anti-rollback state".into()));
    }
    for entry in state.components.values() {
        if entry.highest_sequence == 0 {
            return Err(UpdateError::Security("invalid trusted sequence".into()));
        }
        if let Some(pending) = &entry.pending {
            validate_digest(&pending.manifest_sha256)?;
            if pending.sequence != entry.highest_sequence || pending.accepted_unix <= 0 {
                return Err(UpdateError::Security(
                    "pending acceptance is not the highest sequence".into(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_digest(value: &str) -> Result<()> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(UpdateError::Security("invalid manifest digest".into()));
    }
    Ok(())
}

#[cfg(unix)]
pub fn validate_root_owned_file(path: &Path, max_bytes: u64) -> Result<()> {
    validate_owned_path(path, 0, false)?;
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file()
        || metadata.len() == 0
        || metadata.len() > max_bytes
        || metadata.mode() & 0o022 != 0
    {
        return Err(UpdateError::Security(
            "root-owned file type, size, or mode is unsafe".into(),
        ));
    }
    Ok(())
}

#[cfg(unix)]
pub fn validate_root_owned_directory(path: &Path) -> Result<()> {
    validate_owned_path(path, 0, true)
}

#[cfg(not(unix))]
pub fn validate_root_owned_file(_path: &Path, _max_bytes: u64) -> Result<()> {
    Err(UpdateError::Security(
        "root-owned file validation is Linux-only".into(),
    ))
}

#[cfg(not(unix))]
pub fn validate_root_owned_directory(_path: &Path) -> Result<()> {
    Err(UpdateError::Security(
        "root-owned directory validation is Linux-only".into(),
    ))
}

#[cfg(unix)]
fn validate_owned_path(path: &Path, uid: u32, directory: bool) -> Result<()> {
    if !path.is_absolute() {
        return Err(UpdateError::Security(
            "trusted path must be absolute".into(),
        ));
    }
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || metadata.uid() != uid || metadata.mode() & 0o022 != 0 {
        return Err(UpdateError::Security(
            "trusted path owner/mode/symlink check failed".into(),
        ));
    }
    if directory && !metadata.is_dir() {
        return Err(UpdateError::Security(
            "trusted root is not a directory".into(),
        ));
    }
    let mut current = path.parent();
    while let Some(parent) = current {
        let metadata = fs::symlink_metadata(parent)?;
        if metadata.file_type().is_symlink() || metadata.uid() != 0 || metadata.mode() & 0o022 != 0
        {
            return Err(UpdateError::Security(
                "trusted path has an unsafe ancestor".into(),
            ));
        }
        current = parent.parent();
    }
    Ok(())
}

fn secure_open() -> OpenOptions {
    let mut options = OpenOptions::new();
    #[cfg(unix)]
    options
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .mode(0o600);
    options
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_acceptance_requires_exact_digest_and_highest_sequence() {
        let state = SequenceState {
            schema_version: 2,
            components: BTreeMap::from([(
                Component::Server,
                ComponentTrust {
                    highest_sequence: 7,
                    pending: Some(PendingAcceptance {
                        sequence: 7,
                        manifest_sha256: "a".repeat(64),
                        version: Version::parse("0.7.0").unwrap(),
                        accepted_unix: 1_792_396_800,
                    }),
                },
            )]),
        };
        assert!(validate_state(&state).is_ok());
        assert!(validate_digest(&"g".repeat(64)).is_err());
    }
}
