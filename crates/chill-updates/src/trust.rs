use crate::manifest::Component;
use crate::{Result, UpdateError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SequenceState {
    schema_version: u32,
    components: BTreeMap<Component, u64>,
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
                state.components.get(&component).copied().unwrap_or(0),
                false,
            ))
        })
    }

    pub fn advance(&self, component: Component, sequence: u64) -> Result<()> {
        self.with_lock(|state| {
            let current = state.components.get(&component).copied().unwrap_or(0);
            if sequence <= current {
                return Err(UpdateError::Security(
                    "root anti-rollback sequence did not advance".into(),
                ));
            }
            state.components.insert(component, sequence);
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
                schema_version: 1,
                components: BTreeMap::new(),
            },
            Err(error) => return Err(error.into()),
        };
        if state.schema_version != 1 || state.components.len() > 4 {
            return Err(UpdateError::Security("invalid anti-rollback state".into()));
        }
        let (result, changed) = operation(&mut state)?;
        if changed {
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

#[cfg(not(unix))]
pub fn validate_root_owned_file(_path: &Path, _max_bytes: u64) -> Result<()> {
    Err(UpdateError::Security(
        "root-owned file validation is Linux-only".into(),
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
        if metadata.file_type().is_symlink() || metadata.mode() & 0o002 != 0 {
            return Err(UpdateError::Security(
                "trusted path has a symlink or world-writable parent".into(),
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
