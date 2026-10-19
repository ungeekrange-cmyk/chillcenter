use crate::manifest::Component;
use crate::{Result, UpdateError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use time::OffsetDateTime;
use uuid::Uuid;

const MAX_STATE_BYTES: u64 = 1024 * 1024;
const MAX_OPERATIONS: usize = 128;
const CHECK_RATE_LIMIT_SECONDS: i64 = 5 * 60;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PolicyMode {
    Manual,
    Check,
    Automatic,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub mode: PolicyMode,
    pub hour: u8,
    pub minute: u8,
    pub revision: u64,
}

impl Policy {
    pub fn defaults_for(component: Component) -> Self {
        Self {
            mode: if component == Component::Mac {
                PolicyMode::Check
            } else {
                PolicyMode::Automatic
            },
            hour: 2,
            minute: 0,
            revision: 1,
        }
    }

    fn validate(&self) -> Result<()> {
        if self.hour > 23 || self.minute > 59 {
            return Err(UpdateError::Invalid(
                "policy time is outside 00:00-23:59".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IntentKind {
    Check,
    Install,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    DeferredBusy,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub id: Uuid,
    pub component: Component,
    pub kind: IntentKind,
    pub status: OperationStatus,
    pub created_unix: i64,
    pub updated_unix: i64,
    pub attempts: u8,
    pub result_code: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentState {
    pub highest_sequence: u64,
    pub last_check_unix: Option<i64>,
    pub last_scheduled_local_date: Option<String>,
    pub retry_count: u8,
    pub retry_after_unix: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PersistentState {
    pub schema_version: u32,
    pub policies: BTreeMap<Component, Policy>,
    pub components: BTreeMap<Component, ComponentState>,
    pub operations: Vec<Operation>,
}

impl Default for PersistentState {
    fn default() -> Self {
        let mut policies = BTreeMap::new();
        let mut components = BTreeMap::new();
        for component in [
            Component::Mac,
            Component::Server,
            Component::Web,
            Component::Themes,
        ] {
            policies.insert(component, Policy::defaults_for(component));
            components.insert(component, ComponentState::default());
        }
        Self {
            schema_version: 1,
            policies,
            components,
            operations: Vec::new(),
        }
    }
}

pub struct StateStore {
    root: PathBuf,
    state_path: PathBuf,
    lock_path: PathBuf,
    owner_uid: u32,
    owner_gid: u32,
}

impl StateStore {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        #[cfg(unix)]
        let owner = unsafe { (libc::geteuid(), libc::getegid()) };
        #[cfg(not(unix))]
        let owner = (0, 0);
        Self::open_with_owner(root, owner.0, owner.1, true)
    }

    /// Open service-owned state from the root helper without transferring
    /// ownership to root during atomic rewrites.
    pub fn open_for_owner(root: impl AsRef<Path>, owner_uid: u32, owner_gid: u32) -> Result<Self> {
        Self::open_with_owner(root, owner_uid, owner_gid, false)
    }

    fn open_with_owner(
        root: impl AsRef<Path>,
        owner_uid: u32,
        owner_gid: u32,
        create: bool,
    ) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        if create {
            fs::create_dir_all(&root)?;
        }
        reject_symlink(&root)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let metadata = fs::metadata(&root)?;
            if metadata.uid() != owner_uid || metadata.mode() & 0o022 != 0 {
                return Err(UpdateError::Security(
                    "state root owner/mode mismatch".into(),
                ));
            }
        }
        Ok(Self {
            state_path: root.join("state.json"),
            lock_path: root.join("state.lock"),
            root,
            owner_uid,
            owner_gid,
        })
    }

    pub fn read(&self) -> Result<PersistentState> {
        self.with_locked_state(|state| Ok((state.clone(), false)))
    }

    pub fn compare_and_set_policy(
        &self,
        component: Component,
        expected_revision: u64,
        mode: PolicyMode,
        hour: u8,
        minute: u8,
    ) -> Result<Policy> {
        self.with_locked_state(|state| {
            let current = state
                .policies
                .entry(component)
                .or_insert_with(|| Policy::defaults_for(component));
            if current.revision != expected_revision {
                return Err(UpdateError::Busy(format!(
                    "policy revision conflict; current={}",
                    current.revision
                )));
            }
            let updated = Policy {
                mode,
                hour,
                minute,
                revision: current.revision.saturating_add(1),
            };
            updated.validate()?;
            *current = updated.clone();
            Ok((updated, true))
        })
    }

    pub fn admit_intent(
        &self,
        component: Component,
        kind: IntentKind,
        now: OffsetDateTime,
    ) -> Result<Operation> {
        self.with_locked_state(|state| {
            if let Some(existing) = state.operations.iter().rev().find(|operation| {
                operation.component == component
                    && operation.kind == kind
                    && matches!(
                        operation.status,
                        OperationStatus::Pending
                            | OperationStatus::Running
                            | OperationStatus::DeferredBusy
                    )
            }) {
                return Ok((existing.clone(), false));
            }
            if kind == IntentKind::Install
                && state.operations.iter().any(|operation| {
                    operation.kind == IntentKind::Install
                        && operation.status == OperationStatus::Running
                })
            {
                return Err(UpdateError::Busy(
                    "another component install is running".into(),
                ));
            }
            if kind == IntentKind::Check {
                let component_state = state.components.entry(component).or_default();
                if let Some(last) = component_state.last_check_unix
                    && now.unix_timestamp().saturating_sub(last) < CHECK_RATE_LIMIT_SECONDS
                {
                    if let Some(cached) = state.operations.iter().rev().find(|operation| {
                        operation.component == component && operation.kind == IntentKind::Check
                    }) {
                        return Ok((cached.clone(), false));
                    }
                    return Err(UpdateError::Busy("check rate limit active".into()));
                }
                component_state.last_check_unix = Some(now.unix_timestamp());
            }
            let operation = Operation {
                id: Uuid::new_v4(),
                component,
                kind,
                status: OperationStatus::Pending,
                created_unix: now.unix_timestamp(),
                updated_unix: now.unix_timestamp(),
                attempts: 0,
                result_code: None,
            };
            state.operations.push(operation.clone());
            trim_operations(state);
            Ok((operation, true))
        })
    }

    pub fn transition(
        &self,
        id: Uuid,
        expected: OperationStatus,
        next: OperationStatus,
        result_code: Option<&str>,
        now: OffsetDateTime,
    ) -> Result<Operation> {
        self.with_locked_state(|state| {
            if next == OperationStatus::Running
                && state.operations.iter().any(|operation| {
                    operation.id != id
                        && operation.kind == IntentKind::Install
                        && operation.status == OperationStatus::Running
                })
            {
                return Err(UpdateError::Busy("global install slot is occupied".into()));
            }
            let operation = state
                .operations
                .iter_mut()
                .find(|operation| operation.id == id)
                .ok_or_else(|| UpdateError::Invalid("unknown operation id".into()))?;
            if operation.status != expected {
                return Err(UpdateError::Busy("operation state changed".into()));
            }
            if result_code.is_some_and(|value| {
                value.len() > 128
                    || !value
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || b"_-.:".contains(&byte))
            }) {
                return Err(UpdateError::Invalid("result code is not bounded".into()));
            }
            operation.status = next;
            operation.updated_unix = now.unix_timestamp();
            operation.attempts = operation
                .attempts
                .saturating_add(u8::from(next == OperationStatus::Running));
            operation.result_code = result_code.map(str::to_owned);
            Ok((operation.clone(), true))
        })
    }

    pub fn record_sequence(&self, component: Component, sequence: u64) -> Result<()> {
        self.with_locked_state(|state| {
            let component_state = state.components.entry(component).or_default();
            if sequence <= component_state.highest_sequence {
                return Err(UpdateError::Security(
                    "release sequence did not advance".into(),
                ));
            }
            component_state.highest_sequence = sequence;
            Ok(((), true))
        })
    }

    pub fn update_scheduler<F, T>(&self, update: F) -> Result<T>
    where
        F: FnOnce(&mut PersistentState) -> Result<T>,
    {
        self.with_locked_state(|state| update(state).map(|value| (value, true)))
    }

    fn with_locked_state<T>(
        &self,
        operation: impl FnOnce(&mut PersistentState) -> Result<(T, bool)>,
    ) -> Result<T> {
        let lock = nofollow_options()
            .create(true)
            .read(true)
            .write(true)
            .open(&self.lock_path)?;
        set_service_ownership(&lock, self.owner_uid, self.owner_gid)?;
        lock.lock()?;
        let mut state = self.load_unlocked()?;
        validate_state(&state)?;
        let (value, changed) = operation(&mut state)?;
        if changed {
            self.save_unlocked(&state)?;
        }
        lock.unlock()?;
        Ok(value)
    }

    fn load_unlocked(&self) -> Result<PersistentState> {
        match nofollow_options().read(true).open(&self.state_path) {
            Ok(file) => {
                if file.metadata()?.len() > MAX_STATE_BYTES {
                    return Err(UpdateError::Security("state file is too large".into()));
                }
                let mut bytes = Vec::new();
                file.take(MAX_STATE_BYTES + 1).read_to_end(&mut bytes)?;
                crate::parse_strict_json(&bytes)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(PersistentState::default())
            }
            Err(error) => Err(error.into()),
        }
    }

    fn save_unlocked(&self, state: &PersistentState) -> Result<()> {
        validate_state(state)?;
        let bytes = serde_json::to_vec_pretty(state)?;
        if bytes.len() as u64 > MAX_STATE_BYTES {
            return Err(UpdateError::Security(
                "serialized state is too large".into(),
            ));
        }
        let temporary = self.root.join(format!(".state.{}.tmp", Uuid::new_v4()));
        let mut file = nofollow_options()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        set_service_ownership(&file, self.owner_uid, self.owner_gid)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, &self.state_path)?;
        File::open(&self.root)?.sync_all()?;
        Ok(())
    }
}

fn trim_operations(state: &mut PersistentState) {
    while state.operations.len() > MAX_OPERATIONS {
        if let Some(index) = state.operations.iter().position(|operation| {
            matches!(
                operation.status,
                OperationStatus::Succeeded | OperationStatus::Failed
            )
        }) {
            state.operations.remove(index);
        } else {
            break;
        }
    }
}

fn validate_state(state: &PersistentState) -> Result<()> {
    if state.schema_version != 1 || state.operations.len() > MAX_OPERATIONS {
        return Err(UpdateError::Security(
            "unsupported or unbounded persistent state".into(),
        ));
    }
    for policy in state.policies.values() {
        policy.validate()?;
    }
    if state.policies.len() > 4 || state.components.len() > 4 {
        return Err(UpdateError::Security(
            "persistent component maps are unbounded".into(),
        ));
    }
    for operation in &state.operations {
        if operation
            .result_code
            .as_ref()
            .is_some_and(|value| value.len() > 128)
            || operation.attempts > 16
        {
            return Err(UpdateError::Security(
                "persisted operation is outside bounds".into(),
            ));
        }
    }
    Ok(())
}

fn nofollow_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .mode(0o600);
    }
    options
}

fn set_service_ownership(file: &File, uid: u32, gid: u32) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let metadata = file.metadata()?;
        if (metadata.uid() != uid || metadata.gid() != gid)
            && unsafe { libc::fchown(file.as_raw_fd(), uid, gid) } != 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

fn reject_symlink(path: &Path) -> Result<()> {
    if fs::symlink_metadata(path)?.file_type().is_symlink() {
        return Err(UpdateError::Security(
            "state root cannot be a symlink".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_cas_and_intent_coalescing_are_persistent() {
        let temp = tempfile::tempdir().unwrap();
        let store = StateStore::open(temp.path()).unwrap();
        let updated = store
            .compare_and_set_policy(Component::Server, 1, PolicyMode::Check, 3, 15)
            .unwrap();
        assert_eq!(updated.revision, 2);
        assert!(
            store
                .compare_and_set_policy(Component::Server, 1, PolicyMode::Manual, 2, 0)
                .is_err()
        );
        let now = OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap();
        let first = store
            .admit_intent(Component::Server, IntentKind::Check, now)
            .unwrap();
        let duplicate = store
            .admit_intent(Component::Server, IntentKind::Check, now)
            .unwrap();
        assert_eq!(first.id, duplicate.id);
        let reopened = StateStore::open(temp.path()).unwrap();
        assert_eq!(reopened.read().unwrap().operations.len(), 1);
    }
}
