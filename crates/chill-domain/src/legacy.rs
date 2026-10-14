//! Read-only Cinéfolio 0.5 snapshot conversion.
//!
//! The caller must first create the SQLite backup with `snapshot::coherent_snapshot`.
//! This module never opens the live source through `Store`, so it cannot migrate or
//! otherwise write the original database.
use crate::{
    import::{
        ArchiveDocument, ConflictPolicy, ImportMapping, PortableMedia, PortablePersonal,
        PortablePlaylist, PortableState, ProfileTarget,
    },
    Preferences, Profile, Store,
};
use chill_catalog::{
    database::Database,
    model::{Collection, Media},
};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde_json::Value;
use std::{collections::BTreeMap, path::Path};
use uuid::Uuid;

const MAX_MEDIA: usize = 100_000;
const MAX_COLLECTIONS: usize = 1_000;
const MAX_MEMBERS: usize = 1_000_000;
const MAX_ROW: usize = 1024 * 1024;
const MAX_DOCUMENT: usize = 64 * 1024 * 1024;

fn invalid() -> String {
    "LEGACY_EXPORT_INVALID".into()
}
fn db(_: rusqlite::Error) -> String {
    invalid()
}

fn table(connection: &Connection, name: &str) -> Result<bool, String> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
            [name],
            |r| r.get(0),
        )
        .map_err(db)
}
fn copied_poster(
    media: &Media,
    copy: &mut impl FnMut(&Path) -> Result<String, String>,
) -> Result<Option<String>, String> {
    let Some(poster) = media.poster.as_deref() else {
        return Ok(None);
    };
    let path = Path::new(poster);
    // Provider/NAS locations are metadata only.  Only an explicit custom image
    // can cross the boundary, and the controlled cache callback validates/copies it.
    if !media.poster_custom || !path.is_absolute() || path == Path::new(&media.path) {
        return Ok(None);
    }
    copy(path).map(Some)
}

/// Convert one *already coherent* legacy snapshot into an empty Store schema 3.
/// The returned document is the exact portable document applied to `scratch` and
/// can be handed to the archive builder for a normal versioned export.
pub fn convert_cinefolio_snapshot(
    snapshot: &Path,
    scratch: &Store,
    mut copy_image: impl FnMut(&Path) -> Result<String, String>,
) -> Result<ArchiveDocument, String> {
    let source =
        Connection::open_with_flags(snapshot, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(db)?;
    let version: u32 = source
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(db)?;
    if version > 1 {
        return Err("LEGACY_SCHEMA_UNSUPPORTED".into());
    }
    if source
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM media WHERE length(data)>?1)",
            [MAX_ROW as i64],
            |r| r.get::<_, bool>(0),
        )
        .map_err(db)?
    {
        return Err(invalid());
    }
    // Exercise the dedicated read-only catalogue boundary; unlike Database::open it
    // never creates tables, changes WAL mode, or writes a schema marker.
    let readonly = Database::open_readonly(snapshot)?;
    let listed = readonly.list()?;
    if listed.len() > MAX_MEDIA {
        return Err(invalid());
    }
    let mut rows = Vec::with_capacity(listed.len());
    let mut poster_paths = BTreeMap::new();
    let mut source_by_id = BTreeMap::new();
    for media in listed {
        if media.id.len() > MAX_ROW || !crate::import::relative(&media.id, false) {
            return Err(invalid());
        }
        source_by_id.insert(media.id.clone(), media);
    }
    if source_by_id.len() > MAX_MEDIA {
        return Err(invalid());
    }
    for (legacy, media) in &source_by_id {
        let copied = copied_poster(media, &mut copy_image)?;
        let reference = copied.map(|path| {
            let reference = format!("posters/{}.bin", Uuid::new_v4());
            poster_paths.insert(reference.clone(), path);
            reference
        });
        let mut data = serde_json::to_value(media).map_err(|_| invalid())?;
        let object = data.as_object_mut().ok_or_else(invalid)?;
        object.remove("path");
        object.remove("watched");
        object.remove("rating");
        object.remove("saga");
        object.insert("id".into(), Value::String(legacy.clone()));
        object.insert(
            "poster".into(),
            reference.map(Value::String).unwrap_or(Value::Null),
        );
        object.insert(
            "poster_custom".into(),
            Value::Bool(media.poster_custom && object.get("poster").is_some_and(|v| !v.is_null())),
        );
        rows.push(PortableMedia {
            id: Uuid::new_v4().to_string(),
            relative_path: legacy.clone(),
            data,
        });
    }
    let ids: BTreeMap<_, _> = rows
        .iter()
        .map(|r| (r.relative_path.clone(), r.id.clone()))
        .collect();
    let mut sagas = Vec::new();
    let has_collections = table(&source, "collections")? && table(&source, "collection_members")?;
    if has_collections {
        let mut statement = source
            .prepare("SELECT id,name FROM collections WHERE kind='saga' ORDER BY id")
            .map_err(db)?;
        let result = statement
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
            .map_err(db)?;
        for row in result {
            let (id, name) = row.map_err(db)?;
            if sagas.len() >= MAX_COLLECTIONS || name.len() > MAX_ROW {
                return Err(invalid());
            }
            let sql = if table(&source, "collection_positions")? {
                "SELECT m.media_id FROM collection_members m LEFT JOIN collection_positions p ON p.collection_id=m.collection_id AND p.media_id=m.media_id WHERE m.collection_id=?1 ORDER BY p.position IS NULL,p.position,m.rowid"
            } else {
                "SELECT media_id FROM collection_members WHERE collection_id=?1 ORDER BY rowid"
            };
            let mut members = source.prepare(sql).map_err(db)?;
            let values = members
                .query_map([id], |r| r.get::<_, String>(0))
                .map_err(db)?;
            let mut media_ids = Vec::new();
            for value in values {
                let legacy = value.map_err(db)?;
                media_ids.push(ids.get(&legacy).cloned().ok_or_else(invalid)?);
            }
            if media_ids.len() > MAX_MEMBERS {
                return Err(invalid());
            }
            let poster = if table(&source, "collection_artwork")? {
                source
                    .query_row(
                        "SELECT poster FROM collection_artwork WHERE collection_id=?1",
                        [id],
                        |r| r.get::<_, String>(0),
                    )
                    .optional()
                    .map_err(db)?
                    .map(|p| {
                        let reference = format!("posters/saga-{}.bin", Uuid::new_v4());
                        let cached = copy_image(Path::new(&p))?;
                        poster_paths.insert(reference.clone(), cached);
                        Ok::<String, String>(reference)
                    })
                    .transpose()?
            } else {
                None
            };
            sagas.push(Collection {
                id: id.to_string(),
                name,
                kind: "saga".into(),
                media_ids,
                poster,
            });
        }
    }
    // Old databases without explicit collections retain the historical single saga.
    if !has_collections {
        let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (legacy, media) in &source_by_id {
            if let Some(name) = &media.saga {
                grouped
                    .entry(name.clone())
                    .or_default()
                    .push(ids[legacy].clone());
            }
        }
        sagas = grouped
            .into_iter()
            .enumerate()
            .map(|(n, (name, media_ids))| Collection {
                id: (n + 1).to_string(),
                name,
                kind: "saga".into(),
                media_ids,
                poster: None,
            })
            .collect();
    }
    let profile_id = Uuid::new_v4().to_string();
    let states = source_by_id
        .iter()
        .filter_map(|(legacy, m)| {
            (m.watched || m.rating.is_some()).then(|| PortableState {
                media_id: ids[legacy].clone(),
                position_ms: 0,
                final_watched_ms: if m.watched { 10_000 } else { 0 },
                watched: m.watched,
                rating: m.rating,
            })
        })
        .collect();
    let mut playlists = Vec::new();
    if has_collections {
        let mut statement = source
            .prepare("SELECT id,name FROM collections WHERE kind='playlist' ORDER BY id")
            .map_err(db)?;
        for result in statement
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
            .map_err(db)?
        {
            let (id, name) = result.map_err(db)?;
            if playlists.len() + sagas.len() >= MAX_COLLECTIONS {
                return Err(invalid());
            }
            let mut members = source
                .prepare(
                    "SELECT media_id FROM collection_members WHERE collection_id=?1 ORDER BY rowid",
                )
                .map_err(db)?;
            let media_ids = members
                .query_map([id], |r| r.get::<_, String>(0))
                .map_err(db)?
                .map(|r| ids.get(&r.map_err(db)?).cloned().ok_or_else(invalid))
                .collect::<Result<Vec<_>, _>>()?;
            if media_ids.len() > 1_000 {
                return Err(invalid());
            }
            playlists.push(PortablePlaylist {
                id: Uuid::new_v4().to_string(),
                name,
                media_ids,
            });
        }
    }
    let document = ArchiveDocument {
        format_version: 1,
        export_id: Uuid::new_v4().to_string(),
        profiles: vec![Profile {
            id: profile_id.clone(),
            name: "Cinéfolio".into(),
            revision: 1,
        }],
        media: rows,
        sagas,
        personal: vec![PortablePersonal {
            profile_id: profile_id.clone(),
            preferences: Preferences::default(),
            speeds: vec![],
            states,
            playlists,
        }],
        leases: vec![],
    };
    document.validate()?;
    if serde_json::to_vec(&document).map_err(|_| invalid())?.len() > MAX_DOCUMENT {
        return Err(invalid());
    }
    let mapping = ImportMapping {
        profile_map: BTreeMap::from([(
            profile_id,
            ProfileTarget::New {
                name: "Cinéfolio".into(),
            },
        )]),
        root_prefix: String::new(),
        conflict_policy: ConflictPolicy::ReplaceConflicting,
    };
    let plan = scratch.prepare_portable_import(document.clone(), &mapping, Path::new("/legacy"))?;
    scratch.apply_portable_import(&plan, &poster_paths)?;
    for row in &document.media {
        let _ = scratch.stable_id(&row.relative_path)?;
    }
    Ok(document)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chill_catalog::database::Database;
    use std::fs;

    fn media(id: &str, title: &str, watched: bool, rating: Option<u8>) -> Media {
        Media {
            id: id.into(),
            path: format!("/Volumes/NAS/{id}"),
            title: title.into(),
            category: "film".into(),
            year: Some(2000),
            series: None,
            season: None,
            episode: None,
            genres: vec!["Drama".into()],
            overview: "kept".into(),
            poster: Some(format!("/Volumes/NAS/{id}")),
            poster_custom: false,
            runtime_minutes: Some(90),
            cast: vec!["A".into()],
            directors: vec!["B".into()],
            series_overview: String::new(),
            series_metadata: None,
            series_manual: false,
            rating,
            saga: Some("Fallback".into()),
            watched,
            available: true,
            added_at: 1,
            tmdb_id: Some(42),
        }
    }

    #[test]
    fn read_only_legacy_snapshot_roundtrips_common_and_synthetic_personal_data() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("cinefolio.sqlite");
        let db = Database::open(&source).unwrap();
        db.put(&media("one.mkv", "One", true, Some(5))).unwrap();
        db.put(&media("two.mkv", "Two", false, Some(3))).unwrap();
        drop(db);
        let connection = Connection::open(&source).unwrap();
        connection.execute_batch("CREATE TABLE IF NOT EXISTS collections (id INTEGER PRIMARY KEY, name TEXT NOT NULL, kind TEXT NOT NULL); CREATE TABLE IF NOT EXISTS collection_members (collection_id INTEGER,media_id TEXT); CREATE TABLE IF NOT EXISTS collection_positions (collection_id INTEGER,media_id TEXT,position INTEGER); INSERT INTO collections(id,name,kind) VALUES(1,'Saga','saga'),(2,'Soirée','playlist'); INSERT INTO collection_members VALUES(1,'two.mkv'),(1,'one.mkv'),(2,'one.mkv'),(2,'two.mkv'); INSERT INTO collection_positions VALUES(1,'two.mkv',0),(1,'one.mkv',1);").unwrap();
        drop(connection);
        let before = fs::read(&source).unwrap();
        let modified = fs::metadata(&source).unwrap().modified().unwrap();
        let scratch_path = temp.path().join("scratch.sqlite");
        let mut scratch = Store::open(&scratch_path).unwrap();
        let callback_calls = std::cell::Cell::new(0);
        let document = convert_cinefolio_snapshot(&source, &scratch, |_| {
            callback_calls.set(callback_calls.get() + 1);
            Err("NAS must not be copied".into())
        })
        .unwrap();
        assert_eq!(callback_calls.get(), 0);
        assert_eq!(before, fs::read(&source).unwrap());
        assert_eq!(modified, fs::metadata(&source).unwrap().modified().unwrap());
        assert_eq!(document.profiles[0].name, "Cinéfolio");
        assert_eq!(document.sagas[0].media_ids.len(), 2);
        let exported = scratch.export_json().unwrap();
        let portable: ArchiveDocument = serde_json::from_value(exported).unwrap();
        portable.validate().unwrap();
        let target = Store::open(&temp.path().join("target.sqlite")).unwrap();
        let profile = portable.profiles[0].id.clone();
        let mapping = ImportMapping {
            profile_map: BTreeMap::from([(
                profile,
                ProfileTarget::New {
                    name: "Imported".into(),
                },
            )]),
            root_prefix: String::new(),
            conflict_policy: ConflictPolicy::ReplaceConflicting,
        };
        let plan = target
            .prepare_portable_import(portable, &mapping, Path::new("/legacy"))
            .unwrap();
        target
            .apply_portable_import(&plan, &BTreeMap::new())
            .unwrap();
        assert_eq!(target.profiles().unwrap()[0].name, "Imported");
    }
}
