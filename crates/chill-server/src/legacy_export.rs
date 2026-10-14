//! Offline Cinéfolio exporter. The live source is opened only by SQLite backup's read-only boundary.
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Component, Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
};
const MAX_DB: u64 = 256 * 1024 * 1024;
const MAX_IMAGE: u64 = 8 * 1024 * 1024;
const MAX_IMAGES: u64 = 1024 * 1024 * 1024;
fn invalid() -> String {
    "Export Cinéfolio refusé : chemins, données ou limites invalides. Aucune vidéo ni base source modifiée.".into()
}
fn absolute(path: &Path) -> bool {
    path.is_absolute()
        && path != Path::new("/")
        && !path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
}
struct Workspace {
    root: PathBuf,
    files: Vec<PathBuf>,
}
impl Workspace {
    fn new(parent: &Path) -> Result<Self, String> {
        let root = parent.join(format!(".chill-export-{}", uuid::Uuid::new_v4()));
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&root)
            .map_err(|_| invalid())?;
        Ok(Self {
            root,
            files: vec![],
        })
    }
    fn own(&mut self, path: PathBuf) -> PathBuf {
        self.files.push(path.clone());
        path
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        for file in self.files.iter().rev() {
            let _ = fs::remove_file(file);
        }
        for directory in ["posters", "exports", ""] {
            let _ = fs::remove_dir(self.root.join(directory));
        }
    }
}

/// Output is new, private, atomically published with a no-replace hard link.
/// No network, server, config loading, token, TLS generation or media scan occurs.
pub fn export(source: &Path, image_cache: &Path, output: &Path) -> Result<(), String> {
    if !absolute(source)
        || !absolute(image_cache)
        || !absolute(output)
        || source == output
        || fs::symlink_metadata(output).is_ok()
    {
        return Err(invalid());
    }
    let parent = output.parent().ok_or_else(invalid)?;
    if !fs::metadata(parent).map_err(|_| invalid())?.is_dir() {
        return Err(invalid());
    }
    // Reject symlink components and special source files before SQLite opens it.
    let (source_fd, _) =
        crate::media_stream::open_indexed(source.parent().ok_or_else(invalid)?, source)
            .map_err(|_| invalid())?;
    if source_fd.metadata().map_err(|_| invalid())?.len() > MAX_DB {
        return Err(invalid());
    }
    let mut work = Workspace::new(parent)?;
    let flag = Arc::new(AtomicBool::new(false));
    let snapshot = work.own(work.root.join("source.sqlite"));
    chill_domain::snapshot::coherent_snapshot(source, &snapshot, &flag, MAX_DB)?;
    let scratch = work.own(work.root.join("catalog.sqlite"));
    for name in [
        "catalog.sqlite-wal",
        "catalog.sqlite-shm",
        "source.sqlite-wal",
        "source.sqlite-shm",
        "converted.sqlite-wal",
        "converted.sqlite-shm",
    ] {
        work.own(work.root.join(name));
    }
    let store = chill_domain::Store::open(&scratch)?;
    let poster_root = work.root.join("posters");
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&poster_root)
        .map_err(|_| invalid())?;
    let mut image_total = 0u64;
    chill_domain::legacy::convert_cinefolio_snapshot(&snapshot, &store, |path| {
        let (mut file, version) =
            crate::media_stream::open_indexed(image_cache, path).map_err(|_| invalid())?;
        if version.size > MAX_IMAGE || image_total.saturating_add(version.size) > MAX_IMAGES {
            return Err(invalid());
        }
        let mut bytes = Vec::with_capacity(version.size as usize);
        Read::by_ref(&mut file)
            .take(MAX_IMAGE + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| invalid())?;
        let after = crate::media_stream::FileVersion::from_metadata(
            &file.metadata().map_err(|_| invalid())?,
        )
        .map_err(|_| invalid())?;
        if bytes.len() as u64 != version.size || after != version {
            return Err(invalid());
        }
        chill_catalog::poster::validate(&bytes)?;
        let destination = work.own(poster_root.join(format!("{}.bin", uuid::Uuid::new_v4())));
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&destination)
            .map_err(|_| invalid())?;
        output
            .write_all(&bytes)
            .and_then(|_| output.sync_all())
            .map_err(|_| invalid())?;
        image_total += version.size;
        Ok(destination.to_string_lossy().into_owned())
    })?;
    drop(store);
    let coherent = work.own(work.root.join("converted.sqlite"));
    chill_domain::snapshot::coherent_snapshot(&scratch, &coherent, &flag, MAX_DB)?;
    let id = uuid::Uuid::new_v4().to_string();
    for ext in ["partial", "zip", "data.partial"] {
        work.own(work.root.join("exports").join(format!("{id}.{ext}")));
    }
    crate::archive_api::build_export_files(&work.root, id.clone(), &coherent, flag)
        .map_err(|e| format!("Export Cinéfolio refusé : {}", e.1))?;
    let ready = work.root.join("exports").join(format!("{id}.zip"));
    fs::hard_link(&ready, output).map_err(|_| {
        "Destination déjà présente ou publication impossible. Aucun fichier existant remplacé."
            .to_string()
    })?;
    // A failed directory fsync is an uncertain durability outcome, never a reason
    // to delete a published user's archive. Report it and preserve the archive.
    fs::File::open(parent).and_then(|d|d.sync_all()).map_err(|_|"Archive publiée mais durabilité non confirmée ; vérifiez le fichier avant de réessayer.".to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chill_catalog::{database::Database, model::Media};
    use chill_domain::{
        Store,
        import::{ArchiveDocument, ConflictPolicy, ImportMapping, ProfileTarget},
    };
    use std::collections::BTreeMap;
    fn fixture(id: &str, poster: String) -> Media {
        Media {
            id: id.into(),
            path: format!("/NAS/{id}"),
            title: "Source".into(),
            category: "film".into(),
            year: Some(2000),
            series: None,
            season: None,
            episode: None,
            genres: vec![],
            overview: "Metadata".into(),
            poster: Some(poster),
            poster_custom: false,
            runtime_minutes: None,
            cast: vec![],
            directors: vec![],
            series_overview: String::new(),
            series_metadata: None,
            series_manual: false,
            rating: Some(4),
            saga: None,
            watched: true,
            available: true,
            added_at: 42,
            tmdb_id: Some(7),
        }
    }
    #[test]
    fn live_wal_export_preserves_source_and_cached_provider_image_and_roundtrips() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let source = root.join("cinefolio.sqlite");
        let cache = root.join("cache");
        fs::create_dir(&cache).unwrap();
        let image = cache.join("provider.png");
        fs::write(&image, include_bytes!("../../../icons/icon.png")).unwrap();
        let db = Database::open(&source).unwrap();
        db.put(&fixture("Films/one.mkv", image.to_string_lossy().into()))
            .unwrap();
        let before = fs::read(&source).unwrap();
        let wal = fs::read(temp.path().join("cinefolio.sqlite-wal")).unwrap();
        let output = root.join("portable.zip");
        export(&source, &cache, &output).unwrap();
        assert_eq!(before, fs::read(&source).unwrap());
        assert_eq!(
            wal,
            fs::read(temp.path().join("cinefolio.sqlite-wal")).unwrap()
        );
        assert!(export(&source, &cache, &output).is_err());
        let mut zip = zip::ZipArchive::new(fs::File::open(&output).unwrap()).unwrap();
        let mut bytes = Vec::new();
        zip.by_name("data.json")
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        let document: ArchiveDocument = serde_json::from_slice(&bytes).unwrap();
        document.validate().unwrap();
        assert_eq!(document.media[0].data["added_at"], 42);
        assert_eq!(document.personal[0].states[0].rating, Some(4));
        assert!(document.personal[0].states[0].watched);
        assert!(
            document.media[0].data["poster"]
                .as_str()
                .unwrap()
                .starts_with("posters/")
        );
        assert!(!String::from_utf8(bytes).unwrap().contains("/NAS/"));
        let destination = Store::open(&temp.path().join("target.sqlite")).unwrap();
        let map = ImportMapping {
            profile_map: BTreeMap::from([(
                document.profiles[0].id.clone(),
                ProfileTarget::New {
                    name: "Choisi".into(),
                },
            )]),
            root_prefix: String::new(),
            conflict_policy: ConflictPolicy::KeepDestination,
        };
        let refs = document.media[0].data["poster"]
            .as_str()
            .unwrap()
            .to_string();
        let plan = destination
            .prepare_portable_import(document, &map, Path::new("/new-root"))
            .unwrap();
        let poster_map = BTreeMap::from([(refs, image.to_string_lossy().into_owned())]);
        destination
            .apply_portable_import(&plan, &poster_map)
            .unwrap();
        assert_eq!(destination.profiles().unwrap()[0].name, "Choisi");
    }
    #[test]
    fn invalid_or_external_image_and_output_link_do_not_change_source() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let source = root.join("cinefolio.sqlite");
        let cache = root.join("cache");
        fs::create_dir(&cache).unwrap();
        let outside = temp.path().join("outside.png");
        fs::write(&outside, include_bytes!("../../../icons/icon.png")).unwrap();
        let db = Database::open(&source).unwrap();
        db.put(&fixture("one.mkv", outside.to_string_lossy().into()))
            .unwrap();
        let before = fs::read(&source).unwrap();
        let output = temp.path().join("out.zip");
        assert!(export(&source, &cache, &output).is_err());
        assert!(!output.exists());
        assert_eq!(before, fs::read(&source).unwrap());
        symlink(temp.path().join("missing"), &output).unwrap();
        assert!(export(&source, &cache, &output).is_err());
        assert!(
            fs::symlink_metadata(&output)
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }
}
