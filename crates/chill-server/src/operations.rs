//! Offline operational backup and crash-recoverable replacement restore.
//!
//! This is deliberately separate from the portable household archive.  It is
//! an operator tool for one configured data root and never reads the video root.

use crate::config::{LibrarySettings, ServerConfig, prepare_data_dir};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::{
        fd::AsRawFd,
        unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    },
    path::{Component, Path, PathBuf},
    sync::atomic::AtomicBool,
};
use uuid::Uuid;

const FORMAT_VERSION: u32 = 1;
const SCHEMA_VERSION: u32 = 3;
const MAX_DATABASE: u64 = 256 * 1024 * 1024;
const MAX_POSTER: u64 = 8 * 1024 * 1024;
const MAX_POSTERS: usize = 20_000;
const MAX_FILES: usize = MAX_POSTERS + 8;
const MAX_TOTAL: u64 = 1024 * 1024 * 1024;
const LOCK_NAME: &str = ".chillcenter-data.lock";
const MANIFEST_NAME: &str = "manifest.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format_version: u32,
    schema_version: u32,
    data_root: PathBuf,
    entries: Vec<ManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestEntry {
    path: String,
    size: u64,
    sha256: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PendingJournal {
    operation_id: Uuid,
}

/// Advisory lock stored in the data tree.  Keeping the descriptor alive keeps
/// the lock across renames, which is essential during publication.
pub struct DataLock {
    file: File,
}

impl DataLock {
    pub fn acquire(data_root: &Path) -> Result<Self, String> {
        let path = data_root.join(LOCK_NAME);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&path)
            .map_err(|_| "Verrou de données inaccessible.")?;
        let metadata = file
            .metadata()
            .map_err(|_| "Verrou de données inaccessible.")?;
        if !metadata.is_file()
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.mode() & 0o177 != 0
        {
            return Err("Verrou de données insuffisamment protégé.".into());
        }
        // SAFETY: flock only observes the valid owned descriptor above.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err("Le service doit être arrêté avant cette opération.".into());
        }
        Ok(Self { file })
    }
}

impl Drop for DataLock {
    fn drop(&mut self) {
        // SAFETY: this descriptor remains valid for the lifetime of DataLock.
        unsafe { libc::flock(self.file.as_raw_fd(), libc::LOCK_UN) };
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreResult {
    pub previous_data: PathBuf,
    pub pre_restore_backup: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryResult {
    pub restored_data: PathBuf,
    pub replacement_candidate: Option<PathBuf>,
}

/// Must be called before opening SQLite or TLS on normal service startup.
pub fn ensure_no_pending(config: &ServerConfig) -> Result<(), String> {
    safe_layout(config)?;
    let journal = journal_path(&config.data_dir)?;
    if journal.exists() {
        return Err("Restauration interrompue : exécutez --recover hors ligne.".into());
    }
    Ok(())
}

/// Create a complete operational backup and publish its manifest last.
pub fn backup(config: &ServerConfig, config_path: &Path) -> Result<PathBuf, String> {
    preflight_existing_root(config)?;
    let _lock = DataLock::acquire(&config.data_dir)?;
    prepare_for_offline_operation(config)?;
    backup_locked(config, config_path)
}

fn backup_locked(config: &ServerConfig, config_path: &Path) -> Result<PathBuf, String> {
    let backups = config.data_dir.join("backups");
    private_dir_all(&backups)?;
    let destination = backups.join(Uuid::new_v4().to_string());
    private_new_dir(&destination)?;

    let mut entries = Vec::new();
    let database = config.data_dir.join("catalog.sqlite");
    let db_destination = destination.join("catalog.sqlite");
    chill_domain::snapshot::coherent_snapshot(
        &database,
        &db_destination,
        &AtomicBool::new(false),
        MAX_DATABASE,
    )?;
    entries.push(entry_for(&destination, "catalog.sqlite")?);

    copy_optional_small(
        &config.data_dir.join("library.json"),
        &destination,
        "library.json",
        64 * 1024,
        &mut entries,
    )?;
    copy_posters(&config.data_dir.join("posters"), &destination, &mut entries)?;
    copy_tls(&config.data_dir.join("tls"), &destination, &mut entries)?;
    copy_exact_private(
        config_path,
        &destination.join("server-config.json"),
        64 * 1024,
    )?;
    entries.push(entry_for(&destination, "server-config.json")?);

    entries.sort_by(|a, b| a.path.cmp(&b.path));
    enforce_totals(&entries)?;
    let manifest = Manifest {
        format_version: FORMAT_VERSION,
        schema_version: SCHEMA_VERSION,
        data_root: config.data_dir.clone(),
        entries,
    };
    let bytes =
        serde_json::to_vec_pretty(&manifest).map_err(|_| "Manifeste de sauvegarde invalide.")?;
    write_new_private(&destination.join(MANIFEST_NAME), &bytes)?;
    sync_dir(&destination)?;
    sync_dir(&backups)?;
    sync_dir(&config.data_dir)?;
    Ok(destination)
}

pub fn restore(
    backup_dir: &Path,
    config: &ServerConfig,
    config_path: &Path,
    confirm_replace: bool,
) -> Result<RestoreResult, String> {
    restore_with_fault(
        backup_dir,
        config,
        config_path,
        confirm_replace,
        FaultPoint::None,
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FaultPoint {
    None,
    AfterOldMove,
    AfterNewPublish,
}

fn restore_with_fault(
    backup_dir: &Path,
    config: &ServerConfig,
    config_path: &Path,
    confirm_replace: bool,
    fault: FaultPoint,
) -> Result<RestoreResult, String> {
    if !confirm_replace {
        return Err(
            "Ajoutez --confirm-replace après avoir vérifié la sauvegarde préalable.".into(),
        );
    }
    preflight_existing_root(config)?;
    let current_lock = DataLock::acquire(&config.data_dir)?;
    prepare_for_offline_operation(config)?;
    let journal = journal_path(&config.data_dir)?;
    if journal.exists() {
        return Err("Restauration interrompue : exécutez --recover hors ligne.".into());
    }

    // Validate every declared byte and prepare the complete replacement before
    // changing the configured data tree.
    let manifest = load_and_validate_manifest(backup_dir, config)?;
    let id = Uuid::new_v4();
    let paths = DerivedPaths::new(&config.data_dir, id)?;
    private_new_dir(&paths.stage)?;
    populate_stage(backup_dir, &paths.stage, &manifest)?;
    let stage_lock = DataLock::acquire(&paths.stage)?;

    // AC-02: the current state is backed up only after the incoming backup has
    // passed all validation and before replacement begins.
    let pre_restore_backup = backup_locked(config, config_path)?;
    let pre_restore_relative = pre_restore_backup
        .strip_prefix(&config.data_dir)
        .map_err(|_| "Chemin de sauvegarde préalable invalide.")?
        .to_owned();
    write_journal(&journal, id)?;

    fs::rename(&config.data_dir, &paths.previous)
        .map_err(|_| "Déplacement des données actuelles impossible.")?;
    sync_dir(&paths.parent)?;
    if fault == FaultPoint::AfterOldMove {
        return Err("FAULT_AFTER_OLD_MOVE".into());
    }

    if let Err(error) = fs::rename(&paths.stage, &config.data_dir) {
        let _ = fs::rename(&paths.previous, &config.data_dir);
        let _ = sync_dir(&paths.parent);
        let _ = remove_journal(&journal);
        return Err(if error.kind() == std::io::ErrorKind::NotFound {
            "Staging de restauration introuvable.".into()
        } else {
            "Publication de la restauration impossible.".into()
        });
    }
    sync_dir(&paths.parent)?;
    if fault == FaultPoint::AfterNewPublish {
        return Err("FAULT_AFTER_NEW_PUBLISH".into());
    }

    if let Err(error) = remove_journal(&journal) {
        rollback_published(&paths, &journal)?;
        return Err(error);
    }
    drop(stage_lock);
    drop(current_lock);
    let pre_restore_backup = paths.previous.join(pre_restore_relative);
    Ok(RestoreResult {
        previous_data: paths.previous,
        pre_restore_backup,
    })
}

/// Roll back only paths derived from the UUID in the parent journal.  A
/// published replacement is retained as a recovery candidate, never deleted.
pub fn recover(config: &ServerConfig, confirm_replace: bool) -> Result<RecoveryResult, String> {
    if !confirm_replace {
        return Err("Ajoutez --confirm-replace pour confirmer le retour hors ligne.".into());
    }
    safe_layout(config)?;
    let journal = journal_path(&config.data_dir)?;
    let pending = read_journal(&journal)?;
    let paths = DerivedPaths::new(&config.data_dir, pending.operation_id)?;
    let previous = require_private_generated_dir(&paths.previous)?;
    let mut current_lock = None;
    if config.data_dir.exists() {
        require_private_generated_dir(&config.data_dir)?;
        current_lock = Some(DataLock::acquire(&config.data_dir)?);
    }
    let _previous_lock = DataLock::acquire(&previous)?;
    let mut stage_lock = None;
    if paths.stage.exists() {
        stage_lock = Some(DataLock::acquire(&paths.stage)?);
    }

    let replacement_candidate = if config.data_dir.exists() {
        if paths.stage.exists() || paths.recovery.exists() {
            return Err("État de récupération ambigu ; aucune donnée modifiée.".into());
        }
        fs::rename(&config.data_dir, &paths.recovery)
            .map_err(|_| "Conservation du remplacement interrompu impossible.")?;
        sync_dir(&paths.parent)?;
        Some(paths.recovery.clone())
    } else {
        if !paths.stage.exists() {
            return Err("Staging de récupération introuvable ; aucune donnée modifiée.".into());
        }
        require_private_generated_dir(&paths.stage)?;
        Some(paths.stage.clone())
    };

    if fs::rename(&paths.previous, &config.data_dir).is_err() {
        if replacement_candidate.is_some() {
            let _ = fs::rename(&paths.recovery, &config.data_dir);
            let _ = sync_dir(&paths.parent);
        }
        return Err("Retour aux données précédentes impossible.".into());
    }
    sync_dir(&paths.parent)?;
    remove_journal(&journal)?;
    drop(stage_lock.take());
    drop(current_lock.take());
    Ok(RecoveryResult {
        restored_data: config.data_dir.clone(),
        replacement_candidate,
    })
}

fn rollback_published(paths: &DerivedPaths, journal: &Path) -> Result<(), String> {
    if paths.recovery.exists() {
        return Err("Récupération requise : candidat déjà présent.".into());
    }
    fs::rename(&paths.data_root, &paths.recovery)
        .map_err(|_| "Récupération requise après publication.")?;
    sync_dir(&paths.parent)?;
    fs::rename(&paths.previous, &paths.data_root)
        .map_err(|_| "Récupération requise après publication.")?;
    sync_dir(&paths.parent)?;
    remove_journal(journal)
}

struct DerivedPaths {
    data_root: PathBuf,
    parent: PathBuf,
    previous: PathBuf,
    stage: PathBuf,
    recovery: PathBuf,
}

impl DerivedPaths {
    fn new(data_root: &Path, id: Uuid) -> Result<Self, String> {
        let parent = data_root
            .parent()
            .ok_or("Racine de données invalide.")?
            .to_owned();
        let name = data_root
            .file_name()
            .and_then(OsStr::to_str)
            .filter(|name| !name.is_empty() && !name.starts_with('.'))
            .ok_or("Nom de racine de données invalide.")?;
        Ok(Self {
            data_root: data_root.to_owned(),
            previous: parent.join(format!(".{name}.pre-restore-{id}")),
            stage: parent.join(format!(".{name}.restore-{id}")),
            recovery: parent.join(format!(".{name}.recovery-{id}")),
            parent,
        })
    }
}

fn journal_path(data_root: &Path) -> Result<PathBuf, String> {
    let parent = data_root.parent().ok_or("Racine de données invalide.")?;
    let name = data_root
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or("Racine de données invalide.")?;
    Ok(parent.join(format!(".{name}.restore-pending.json")))
}

fn write_journal(path: &Path, operation_id: Uuid) -> Result<(), String> {
    let bytes = serde_json::to_vec(&PendingJournal { operation_id })
        .map_err(|_| "Journal de restauration invalide.")?;
    write_new_private(path, &bytes)?;
    sync_dir(path.parent().ok_or("Journal de restauration invalide.")?)
}

fn read_journal(path: &Path) -> Result<PendingJournal, String> {
    let bytes = read_regular_bounded(path, 4096)?;
    serde_json::from_slice(&bytes).map_err(|_| "Journal de restauration invalide.".into())
}

fn remove_journal(path: &Path) -> Result<(), String> {
    fs::remove_file(path).map_err(|_| "Suppression du journal impossible.")?;
    sync_dir(path.parent().ok_or("Journal de restauration invalide.")?)
}

fn prepare_for_offline_operation(config: &ServerConfig) -> Result<(), String> {
    prepare_data_dir(&config.data_dir)?;
    if journal_path(&config.data_dir)?.exists() {
        return Err("Restauration interrompue : exécutez --recover hors ligne.".into());
    }
    Ok(())
}

fn preflight_existing_root(config: &ServerConfig) -> Result<(), String> {
    safe_layout(config)?;
    let metadata = config
        .data_dir
        .symlink_metadata()
        .map_err(|_| "Racine de données opérationnelle introuvable.")?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("Racine de données opérationnelle non sûre.".into());
    }
    Ok(())
}

fn safe_layout(config: &ServerConfig) -> Result<(), String> {
    config.validate()?;
    let root = &config.data_dir;
    if root == Path::new("/") || root.file_name().is_none() {
        return Err("Racine de données opérationnelle dangereuse.".into());
    }
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from)
        && *root == home
    {
        return Err("Le dossier personnel ne peut pas être la racine de données.".into());
    }
    if let Ok(workspace) = std::env::current_dir()
        && (root == &workspace || workspace.starts_with(root) || root.starts_with(&workspace))
    {
        return Err("Le workspace ne peut pas être la racine de données.".into());
    }
    let library = LibrarySettings::load(config)?;
    for video in library.roots() {
        if root.starts_with(&video) || video.starts_with(root) {
            return Err("La racine locale ne peut pas chevaucher un emplacement vidéo/NAS.".into());
        }
    }
    Ok(())
}

fn load_and_validate_manifest(backup: &Path, config: &ServerConfig) -> Result<Manifest, String> {
    require_private_generated_dir(backup)?;
    let bytes = read_regular_bounded(&backup.join(MANIFEST_NAME), 4 * 1024 * 1024)?;
    let manifest: Manifest =
        serde_json::from_slice(&bytes).map_err(|_| "Manifeste de sauvegarde invalide.")?;
    if manifest.format_version != FORMAT_VERSION
        || manifest.schema_version != SCHEMA_VERSION
        || manifest.data_root != config.data_dir
    {
        return Err("Sauvegarde incompatible avec cette racine de données.".into());
    }
    let mut declared = BTreeMap::new();
    let mut destinations = BTreeSet::new();
    for entry in &manifest.entries {
        validate_entry(entry)?;
        if declared.insert(entry.path.clone(), entry).is_some() {
            return Err("Entrées de sauvegarde dupliquées.".into());
        }
        if let Some(destination) = restore_destination(&entry.path)?
            && !destinations.insert(destination)
        {
            return Err("Entrées de sauvegarde non injectives.".into());
        }
        let source = backup.join(&entry.path);
        let metadata = source
            .symlink_metadata()
            .map_err(|_| "Entrée de sauvegarde absente.")?;
        if !metadata.is_file() || metadata.len() != entry.size || sha256(&source)? != entry.sha256 {
            return Err("Sauvegarde corrompue ou modifiée.".into());
        }
    }
    if !declared.contains_key("catalog.sqlite") || !declared.contains_key("server-config.json") {
        return Err("Sauvegarde opérationnelle incomplète.".into());
    }
    enforce_totals(&manifest.entries)?;
    let actual = actual_backup_files(backup)?;
    let expected: BTreeSet<_> = declared
        .keys()
        .cloned()
        .chain([MANIFEST_NAME.to_string()])
        .collect();
    if actual != expected {
        return Err("La sauvegarde contient des fichiers non déclarés.".into());
    }
    Ok(manifest)
}

fn validate_entry(entry: &ManifestEntry) -> Result<(), String> {
    if entry.sha256.len() != 64 || !entry.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Empreinte de sauvegarde invalide.".into());
    }
    let path = Path::new(&entry.path);
    if entry.path.contains('\\')
        || path.is_absolute()
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        || restore_destination(&entry.path)?.is_none() && entry.path != "server-config.json"
    {
        return Err("Chemin de sauvegarde interdit.".into());
    }
    if entry.path.starts_with("posters/") && entry.size > MAX_POSTER {
        return Err("Affiche de sauvegarde trop volumineuse.".into());
    }
    if entry.path == "catalog.sqlite" && entry.size > MAX_DATABASE {
        return Err("Base de sauvegarde trop volumineuse.".into());
    }
    Ok(())
}

fn restore_destination(path: &str) -> Result<Option<String>, String> {
    let mapped = match path {
        "catalog.sqlite" | "library.json" => Some(path.to_string()),
        "secrets/tls/certificate.pem" => Some("tls/certificate.pem".into()),
        "secrets/tls/service-key.pem" => Some("tls/service-key.pem".into()),
        "secrets/tls/schedule.json" => Some("tls/schedule.json".into()),
        "server-config.json" => None,
        value if value.starts_with("posters/") && value.len() > "posters/".len() => {
            Some(value.to_string())
        }
        _ => return Err("Entrée non autorisée dans la sauvegarde.".into()),
    };
    Ok(mapped)
}

fn populate_stage(backup: &Path, stage: &Path, manifest: &Manifest) -> Result<(), String> {
    // SQLite backup API performs quick_check; open_export_snapshot pins schema 3.
    let validation = stage.join(".catalog-validation.sqlite");
    chill_domain::snapshot::coherent_snapshot(
        &backup.join("catalog.sqlite"),
        &validation,
        &AtomicBool::new(false),
        MAX_DATABASE,
    )?;
    chill_domain::Store::open_export_snapshot(&validation)?;
    fs::remove_file(&validation).map_err(|_| "Nettoyage de validation impossible.")?;

    for entry in &manifest.entries {
        let Some(relative) = restore_destination(&entry.path)? else {
            continue;
        };
        let target = stage.join(relative);
        if let Some(parent) = target.parent() {
            private_dir_all(parent)?;
        }
        copy_exact_private(&backup.join(&entry.path), &target, entry.size)?;
        if target
            .metadata()
            .map_err(|_| "Copie restaurée inaccessible.")?
            .len()
            != entry.size
            || sha256(&target)? != entry.sha256
        {
            return Err("La sauvegarde a changé pendant sa préparation.".into());
        }
    }
    sync_tree(stage)?;
    Ok(())
}

fn copy_posters(
    source: &Path,
    backup: &Path,
    entries: &mut Vec<ManifestEntry>,
) -> Result<(), String> {
    if !source.exists() {
        return Ok(());
    }
    let metadata = source
        .symlink_metadata()
        .map_err(|_| "Dossier d'affiches inaccessible.")?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("Dossier d'affiches non sûr.".into());
    }
    let mut files = Vec::new();
    collect_regular_files(source, source, &mut files)?;
    if files.len() > MAX_POSTERS {
        return Err("Trop d'affiches à sauvegarder.".into());
    }
    for relative in files {
        let relative_text = relative.to_str().ok_or("Nom d'affiche invalide.")?;
        let manifest_path = format!("posters/{relative_text}");
        let from = source.join(&relative);
        let size = from.metadata().map_err(|_| "Affiche inaccessible.")?.len();
        if size > MAX_POSTER {
            return Err("Affiche trop volumineuse.".into());
        }
        let to = backup.join(&manifest_path);
        if let Some(parent) = to.parent() {
            private_dir_all(parent)?;
        }
        copy_exact_private(&from, &to, MAX_POSTER)?;
        entries.push(entry_for(backup, &manifest_path)?);
    }
    Ok(())
}

fn collect_regular_files(
    root: &Path,
    current: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let mut children = fs::read_dir(current)
        .map_err(|_| "Dossier de sauvegarde inaccessible.")?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "Dossier de sauvegarde inaccessible.")?;
    children.sort_by_key(|e| e.file_name());
    for child in children {
        let metadata = child
            .path()
            .symlink_metadata()
            .map_err(|_| "Entrée inaccessible.")?;
        if metadata.file_type().is_symlink() {
            return Err("Les liens symboliques sont interdits.".into());
        }
        if metadata.is_dir() {
            collect_regular_files(root, &child.path(), files)?;
        } else if metadata.is_file() {
            files.push(
                child
                    .path()
                    .strip_prefix(root)
                    .map_err(|_| "Chemin invalide.")?
                    .to_owned(),
            );
            if files.len() > MAX_FILES {
                return Err("Trop de fichiers à sauvegarder.".into());
            }
        } else {
            return Err("Type de fichier interdit.".into());
        }
    }
    Ok(())
}

fn copy_tls(source: &Path, backup: &Path, entries: &mut Vec<ManifestEntry>) -> Result<(), String> {
    if !source.exists() {
        return Ok(());
    }
    let directory = source
        .symlink_metadata()
        .map_err(|_| "Secrets TLS inaccessibles.")?;
    if !directory.is_dir()
        || directory.file_type().is_symlink()
        || directory.uid() != unsafe { libc::geteuid() }
        || directory.mode() & 0o077 != 0
    {
        return Err("Dossier TLS insuffisamment protégé.".into());
    }
    let allowed = ["certificate.pem", "service-key.pem", "schedule.json"];
    let actual: BTreeSet<_> = fs::read_dir(source)
        .map_err(|_| "Secrets TLS inaccessibles.")?
        .map(|entry| entry.map(|e| e.file_name()))
        .collect::<Result<_, _>>()
        .map_err(|_| "Secrets TLS inaccessibles.")?;
    let expected: BTreeSet<_> = allowed
        .iter()
        .map(OsStr::new)
        .map(OsStr::to_os_string)
        .collect();
    if actual != expected {
        return Err("Dossier TLS incomplet ou non reconnu.".into());
    }
    for name in allowed {
        let source_path = source.join(name);
        let metadata = source_path
            .symlink_metadata()
            .map_err(|_| "Secret TLS inaccessible.")?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.mode() & 0o077 != 0
        {
            return Err("Secret TLS insuffisamment protégé.".into());
        }
        let relative = format!("secrets/tls/{name}");
        let target = backup.join(&relative);
        private_dir_all(target.parent().unwrap())?;
        copy_exact_private(&source_path, &target, 64 * 1024)?;
        entries.push(entry_for(backup, &relative)?);
    }
    Ok(())
}

fn copy_optional_small(
    source: &Path,
    backup: &Path,
    relative: &str,
    maximum: u64,
    entries: &mut Vec<ManifestEntry>,
) -> Result<(), String> {
    if source.exists() {
        copy_exact_private(source, &backup.join(relative), maximum)?;
        entries.push(entry_for(backup, relative)?);
    }
    Ok(())
}

fn entry_for(root: &Path, relative: &str) -> Result<ManifestEntry, String> {
    let path = root.join(relative);
    Ok(ManifestEntry {
        path: relative.into(),
        size: path
            .metadata()
            .map_err(|_| "Fichier sauvegardé inaccessible.")?
            .len(),
        sha256: sha256(&path)?,
    })
}

fn enforce_totals(entries: &[ManifestEntry]) -> Result<(), String> {
    if entries.len() > MAX_FILES
        || entries
            .iter()
            .try_fold(0u64, |sum, entry| sum.checked_add(entry.size))
            .ok_or("Sauvegarde trop volumineuse.")?
            > MAX_TOTAL
        || entries
            .iter()
            .filter(|e| e.path.starts_with("posters/"))
            .count()
            > MAX_POSTERS
    {
        return Err("Sauvegarde hors limites opérationnelles.".into());
    }
    Ok(())
}

fn actual_backup_files(root: &Path) -> Result<BTreeSet<String>, String> {
    let mut paths = Vec::new();
    collect_regular_files(root, root, &mut paths)?;
    paths
        .into_iter()
        .map(|path| {
            path.to_str()
                .map(str::to_string)
                .ok_or_else(|| "Nom de fichier de sauvegarde invalide.".into())
        })
        .collect()
}

fn sha256(path: &Path) -> Result<String, String> {
    let mut file = open_regular_nofollow(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| "Lecture de sauvegarde impossible.")?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn copy_exact_private(source: &Path, destination: &Path, maximum: u64) -> Result<(), String> {
    let input = open_regular_nofollow(source)?;
    let size = input
        .metadata()
        .map_err(|_| "Fichier source inaccessible.")?
        .len();
    if size > maximum {
        return Err("Fichier opérationnel trop volumineux.".into());
    }
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(destination)
        .map_err(|_| "Destination de sauvegarde inaccessible.")?;
    let copied = std::io::copy(&mut input.take(maximum.saturating_add(1)), &mut output)
        .map_err(|_| "Copie opérationnelle interrompue.")?;
    if copied != size || copied > maximum {
        return Err("Le fichier opérationnel a changé pendant sa copie.".into());
    }
    output
        .sync_all()
        .map_err(|_| "Copie opérationnelle non enregistrée.")?;
    Ok(())
}

fn read_regular_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>, String> {
    let mut file = open_regular_nofollow(path)?;
    if file.metadata().map_err(|_| "Fichier inaccessible.")?.len() > maximum {
        return Err("Fichier opérationnel trop volumineux.".into());
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|_| "Lecture opérationnelle impossible.")?;
    Ok(bytes)
}

fn open_regular_nofollow(path: &Path) -> Result<File, String> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .map_err(|_| "Fichier opérationnel inaccessible.")?;
    if !file
        .metadata()
        .map_err(|_| "Fichier opérationnel inaccessible.")?
        .is_file()
    {
        return Err("Type de fichier opérationnel interdit.".into());
    }
    Ok(file)
}

fn write_new_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .map_err(|_| "Écriture opérationnelle impossible.")?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "Écriture opérationnelle non enregistrée.".into())
}

fn private_new_dir(path: &Path) -> Result<(), String> {
    fs::DirBuilder::new()
        .mode(0o700)
        .create(path)
        .map_err(|_| "Création du dossier opérationnel impossible.".to_string())
}

fn private_dir_all(path: &Path) -> Result<(), String> {
    if path.exists() {
        let metadata = path
            .symlink_metadata()
            .map_err(|_| "Dossier opérationnel inaccessible.")?;
        if !metadata.is_dir()
            || metadata.file_type().is_symlink()
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.mode() & 0o077 != 0
        {
            return Err("Dossier opérationnel insuffisamment protégé.".into());
        }
    }
    fs::create_dir_all(path).map_err(|_| "Création du dossier opérationnel impossible.")?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|_| "Protection du dossier opérationnel impossible.".to_string())
}

fn require_private_generated_dir(path: &Path) -> Result<PathBuf, String> {
    let metadata = path
        .symlink_metadata()
        .map_err(|_| "Dossier opérationnel introuvable.")?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o077 != 0
    {
        return Err("Dossier opérationnel insuffisamment protégé.".into());
    }
    Ok(path.to_owned())
}

fn sync_tree(root: &Path) -> Result<(), String> {
    let mut dirs = vec![root.to_owned()];
    let mut index = 0;
    while index < dirs.len() {
        let current = dirs[index].clone();
        index += 1;
        for entry in fs::read_dir(&current).map_err(|_| "Synchronisation impossible.")? {
            let entry = entry.map_err(|_| "Synchronisation impossible.")?;
            let metadata = entry
                .path()
                .symlink_metadata()
                .map_err(|_| "Synchronisation impossible.")?;
            if metadata.is_dir() {
                dirs.push(entry.path());
            } else if metadata.is_file() {
                File::open(entry.path())
                    .and_then(|f| f.sync_all())
                    .map_err(|_| "Synchronisation impossible.")?;
            } else {
                return Err("Type de fichier interdit.".into());
            }
        }
    }
    for directory in dirs.into_iter().rev() {
        sync_dir(&directory)?;
    }
    Ok(())
}

fn sync_dir(path: &Path) -> Result<(), String> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| "Synchronisation du dossier impossible.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chill_domain::Store;
    use std::os::unix::fs::PermissionsExt;

    fn fixture() -> (tempfile::TempDir, ServerConfig, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().canonicalize().unwrap().join("data");
        prepare_data_dir(&data).unwrap();
        let config = ServerConfig {
            data_dir: data.clone(),
            video_root: Some(temp.path().join("nas")),
            ..ServerConfig::default()
        };
        let config_path = temp.path().join("server.json");
        fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();
        fs::set_permissions(&config_path, fs::Permissions::from_mode(0o600)).unwrap();
        let store = Store::open(&data.join("catalog.sqlite")).unwrap();
        store.create_profile("Courant").unwrap();
        fs::write(
            data.join("library.json"),
            br#"{"video_root":"/srv/nas","revision":4}"#,
        )
        .unwrap();
        let posters = data.join("posters");
        private_dir_all(&posters).unwrap();
        fs::write(posters.join("custom.png"), b"custom-poster").unwrap();
        let tls = data.join("tls");
        private_dir_all(&tls).unwrap();
        for (name, bytes) in [
            ("certificate.pem", b"certificate".as_slice()),
            ("service-key.pem", b"PRIVATE KEY".as_slice()),
            ("schedule.json", b"{}".as_slice()),
        ] {
            fs::write(tls.join(name), bytes).unwrap();
            fs::set_permissions(tls.join(name), fs::Permissions::from_mode(0o600)).unwrap();
        }
        (temp, config, config_path)
    }

    #[test]
    fn live_lock_refuses_backup_and_restore_before_artifacts() {
        let (_temp, config, config_path) = fixture();
        let lock = DataLock::acquire(&config.data_dir).unwrap();
        assert!(backup(&config, &config_path).is_err());
        assert!(!config.data_dir.join("backups").exists());
        drop(lock);
    }

    #[test]
    fn backup_captures_wal_posters_config_and_private_tls_without_nas() {
        let (_temp, config, config_path) = fixture();
        let backup_dir = backup(&config, &config_path).unwrap();
        let manifest = load_and_validate_manifest(&backup_dir, &config).unwrap();
        assert!(manifest.entries.iter().any(|e| e.path == "catalog.sqlite"));
        assert!(
            manifest
                .entries
                .iter()
                .any(|e| e.path == "posters/custom.png")
        );
        assert!(
            manifest
                .entries
                .iter()
                .any(|e| e.path == "secrets/tls/service-key.pem")
        );
        assert!(!manifest.entries.iter().any(|e| e.path.contains("nas")));
        assert_eq!(
            backup_dir
                .join("secrets/tls/service-key.pem")
                .metadata()
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        Store::open_export_snapshot(&backup_dir.join("catalog.sqlite")).unwrap();
    }

    #[test]
    fn corrupt_hash_schema_traversal_and_no_confirmation_leave_target_bytes() {
        let (_temp, config, config_path) = fixture();
        let backup_dir = backup(&config, &config_path).unwrap();
        let before = fs::read(config.data_dir.join("catalog.sqlite")).unwrap();
        assert!(restore(&backup_dir, &config, &config_path, false).is_err());
        fs::write(backup_dir.join("library.json"), b"tampered").unwrap();
        assert!(restore(&backup_dir, &config, &config_path, true).is_err());
        assert_eq!(
            before,
            fs::read(config.data_dir.join("catalog.sqlite")).unwrap()
        );

        let traversal = backup(&config, &config_path).unwrap();
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(traversal.join(MANIFEST_NAME)).unwrap()).unwrap();
        manifest["entries"][0]["path"] = serde_json::json!("../escape");
        fs::write(
            traversal.join(MANIFEST_NAME),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        assert!(restore(&traversal, &config, &config_path, true).is_err());

        let future = backup(&config, &config_path).unwrap();
        let database = future.join("catalog.sqlite");
        let connection = rusqlite::Connection::open(&database).unwrap();
        connection.pragma_update(None, "user_version", 4).unwrap();
        drop(connection);
        let mut manifest: Manifest =
            serde_json::from_slice(&fs::read(future.join(MANIFEST_NAME)).unwrap()).unwrap();
        let database_entry = manifest
            .entries
            .iter_mut()
            .find(|entry| entry.path == "catalog.sqlite")
            .unwrap();
        database_entry.size = database.metadata().unwrap().len();
        database_entry.sha256 = sha256(&database).unwrap();
        fs::write(
            future.join(MANIFEST_NAME),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        assert!(restore(&future, &config, &config_path, true).is_err());
        assert_eq!(
            before,
            fs::read(config.data_dir.join("catalog.sqlite")).unwrap()
        );
    }

    #[test]
    fn valid_restore_preserves_prior_tree_and_exact_operational_bytes() {
        let (_temp, config, config_path) = fixture();
        let generation =
            crate::sync_api::database_generation(&config.data_dir.join("catalog.sqlite")).unwrap();
        let backup_dir = backup(&config, &config_path).unwrap();
        let expected_db = fs::read(backup_dir.join("catalog.sqlite")).unwrap();
        let expected_poster = fs::read(backup_dir.join("posters/custom.png")).unwrap();
        let store = Store::open(&config.data_dir.join("catalog.sqlite")).unwrap();
        store.create_profile("Après").unwrap();
        drop(store);
        let result = restore(&backup_dir, &config, &config_path, true).unwrap();
        assert!(result.previous_data.is_dir());
        assert!(result.pre_restore_backup.starts_with(&result.previous_data));
        assert_eq!(
            expected_db,
            fs::read(config.data_dir.join("catalog.sqlite")).unwrap()
        );
        assert_eq!(
            expected_poster,
            fs::read(config.data_dir.join("posters/custom.png")).unwrap()
        );
        assert_eq!(
            fs::read(config.data_dir.join("tls/service-key.pem")).unwrap(),
            b"PRIVATE KEY"
        );
        assert!(!config.data_dir.join("server-config.json").exists());
        let restored_generation =
            crate::sync_api::database_generation(&config.data_dir.join("catalog.sqlite")).unwrap();
        assert_ne!(
            generation, restored_generation,
            "Restore must invalidate equal SQLite epochs"
        );
        assert_eq!(
            restored_generation,
            crate::sync_api::database_generation(&config.data_dir.join("catalog.sqlite")).unwrap(),
            "Restart must retain the cursor generation"
        );
    }

    #[test]
    fn recovery_handles_both_crash_boundaries_and_keeps_published_candidate() {
        for point in [FaultPoint::AfterOldMove, FaultPoint::AfterNewPublish] {
            let (_temp, config, config_path) = fixture();
            let backup_dir = backup(&config, &config_path).unwrap();
            fs::write(config.data_dir.join("marker"), b"old").unwrap();
            assert!(restore_with_fault(&backup_dir, &config, &config_path, true, point).is_err());
            assert!(ensure_no_pending(&config).is_err());
            let recovered = recover(&config, true).unwrap();
            assert_eq!(fs::read(config.data_dir.join("marker")).unwrap(), b"old");
            assert!(recovered.replacement_candidate.unwrap().is_dir());
            ensure_no_pending(&config).unwrap();
        }
    }

    #[test]
    fn dangerous_roots_are_rejected() {
        let mut config = ServerConfig {
            data_dir: PathBuf::from("/"),
            ..ServerConfig::default()
        };
        assert!(safe_layout(&config).is_err());
        config.data_dir = std::env::current_dir().unwrap();
        assert!(safe_layout(&config).is_err());
        config.data_dir = PathBuf::from("/srv/video/chill");
        config.video_root = Some(PathBuf::from("/srv/video"));
        assert!(safe_layout(&config).is_err());
    }

    #[test]
    fn backup_layout_refuses_data_root_overlapping_an_extra_library_location() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("data");
        fs::create_dir(&data).unwrap();
        let config = ServerConfig {
            data_dir: data.clone(),
            ..ServerConfig::default()
        };
        let settings = LibrarySettings {
            video_root: None,
            additional_video_roots: vec![data.join("videos")],
            revision: 1,
        };
        fs::write(
            data.join("library.json"),
            serde_json::to_vec(&settings).unwrap(),
        )
        .unwrap();
        assert!(safe_layout(&config).is_err());
    }
}
