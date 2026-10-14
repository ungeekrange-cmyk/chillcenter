//! Validated, explicit portable imports. Plans are read-only; one SQLite
//! transaction owns all common and personal writes. No filesystem operations.
use crate::{
    Preferences, Store,
    store::{valid_collection_name, valid_profile_name},
    valid_speed,
};
use chill_catalog::model::{Collection, Media, MediaPatch, SeriesPatch};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProfileTarget {
    Existing { id: String, expected_revision: u64 },
    New { name: String },
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConflictPolicy {
    KeepDestination,
    ReplaceConflicting,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportMapping {
    pub profile_map: BTreeMap<String, ProfileTarget>,
    pub root_prefix: String,
    pub conflict_policy: ConflictPolicy,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveDocument {
    pub format_version: u64,
    pub export_id: String,
    pub profiles: Vec<crate::Profile>,
    pub media: Vec<PortableMedia>,
    pub sagas: Vec<Collection>,
    pub personal: Vec<PortablePersonal>,
    #[serde(default)]
    pub leases: Vec<Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortableMedia {
    pub id: String,
    pub relative_path: String,
    pub data: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortablePersonal {
    pub profile_id: String,
    pub preferences: Preferences,
    pub speeds: Vec<PortableSpeed>,
    pub states: Vec<PortableState>,
    pub playlists: Vec<PortablePlaylist>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortableSpeed {
    pub series_key: String,
    pub speed: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortableState {
    pub media_id: String,
    pub position_ms: u64,
    pub final_watched_ms: u64,
    pub watched: bool,
    pub rating: Option<u8>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortablePlaylist {
    pub id: String,
    pub name: String,
    pub media_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedProfile {
    pub source: String,
    pub id: String,
    pub name: String,
    pub new: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedMedia {
    pub source: String,
    pub id: String,
    pub legacy: String,
    pub data: Media,
    pub existing: bool,
    pub poster_ref: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedSaga {
    pub id: i64,
    pub name: String,
    pub members: Vec<String>,
    pub existing: bool,
    pub poster_ref: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedImport {
    pub export_id: String,
    pub mapping_key: String,
    pub epoch: u64,
    pub policy: ConflictPolicy,
    pub profiles: Vec<PlannedProfile>,
    pub media: Vec<PlannedMedia>,
    pub sagas: Vec<PlannedSaga>,
    pub personal: Vec<PortablePersonal>,
    pub conflicts: Vec<String>,
    pub already_imported: bool,
}
#[derive(Debug, Serialize)]
pub struct ImportResult {
    pub already_imported: bool,
    pub media: usize,
    pub profiles: usize,
    pub epoch: u64,
}
fn invalid() -> String {
    "IMPORT_INVALID".into()
}
fn db(_: rusqlite::Error) -> String {
    "Erreur de base locale. Vos données n’ont pas été remplacées.".into()
}
fn uuid(value: &str) -> Result<(), String> {
    uuid::Uuid::parse_str(value)
        .map(|_| ())
        .map_err(|_| invalid())
}
pub fn relative(value: &str, empty: bool) -> bool {
    (empty && value.is_empty())
        || (!value.is_empty()
            && !value.contains('\\')
            && !value.chars().any(char::is_control)
            && !Path::new(value).is_absolute()
            && Path::new(value)
                .components()
                .all(|c| matches!(c, Component::Normal(_)))
            && value
                .split('/')
                .all(|v| !v.is_empty() && v != "." && v != ".."))
}
fn poster(value: &Option<String>) -> Result<(), String> {
    if value
        .as_ref()
        .is_some_and(|s| !s.starts_with("posters/") || !relative(s, false))
    {
        return Err(invalid());
    }
    Ok(())
}
fn unique_refs(values: &[String], known: &BTreeSet<String>) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    if values.len() > 100_000 || values.iter().any(|v| !known.contains(v) || !seen.insert(v)) {
        return Err(invalid());
    }
    Ok(())
}

impl ArchiveDocument {
    pub fn validate(&self) -> Result<(), String> {
        if self.format_version != 1
            || !self.leases.is_empty()
            || self.media.len() > 100_000
            || self.profiles.len() > 1000
        {
            return Err(invalid());
        }
        uuid(&self.export_id)?;
        let mut profiles = BTreeSet::new();
        for p in &self.profiles {
            uuid(&p.id)?;
            valid_profile_name(&p.name)?;
            if !profiles.insert(p.id.clone()) {
                return Err(invalid());
            }
        }
        let mut media = BTreeSet::new();
        let mut paths = BTreeSet::new();
        for m in &self.media {
            uuid(&m.id)?;
            if !relative(&m.relative_path, false)
                || !media.insert(m.id.clone())
                || !paths.insert(m.relative_path.clone())
            {
                return Err(invalid());
            }
            let _ = checked_media(m, "", &m.relative_path)?;
        }
        let mut saga_names = BTreeSet::new();
        for s in &self.sagas {
            if s.kind != "saga" || !saga_names.insert(valid_collection_name(&s.name)?) {
                return Err(invalid());
            }
            poster(&s.poster)?;
            unique_refs(&s.media_ids, &media)?;
        }
        let mut people = BTreeSet::new();
        for p in &self.personal {
            if !profiles.contains(&p.profile_id) || !people.insert(p.profile_id.clone()) {
                return Err(invalid());
            }
            p.preferences.validate()?;
            if p.preferences
                .audio_language
                .chars()
                .chain(p.preferences.subtitle_language.chars())
                .any(char::is_control)
            {
                return Err(invalid());
            }
            let mut keys = BTreeSet::new();
            for speed in &p.speeds {
                if speed.series_key.is_empty()
                    || speed.series_key.len() > 500
                    || !valid_speed(speed.speed)
                    || !keys.insert(&speed.series_key)
                {
                    return Err(invalid());
                }
            }
            let mut states = BTreeSet::new();
            for s in &p.states {
                if !media.contains(&s.media_id)
                    || !states.insert(&s.media_id)
                    || s.position_ms > i64::MAX as u64
                    || s.final_watched_ms > i64::MAX as u64
                    || s.rating.is_some_and(|n| !(1..=5).contains(&n))
                    || (!s.watched && s.final_watched_ms >= 10_000)
                {
                    return Err(invalid());
                }
            }
            let mut ids = BTreeSet::new();
            let mut names = BTreeSet::new();
            for l in &p.playlists {
                uuid(&l.id)?;
                if !ids.insert(&l.id) || !names.insert(valid_collection_name(&l.name)?) {
                    return Err(invalid());
                }
                unique_refs(&l.media_ids, &media)?;
            }
        }
        if people != profiles {
            return Err(invalid());
        }
        Ok(())
    }
}
fn checked_media(row: &PortableMedia, root: &str, legacy: &str) -> Result<Media, String> {
    const KEYS: &[&str] = &[
        "id",
        "title",
        "category",
        "year",
        "series",
        "season",
        "episode",
        "genres",
        "overview",
        "poster",
        "poster_custom",
        "runtime_minutes",
        "cast",
        "directors",
        "series_overview",
        "series_metadata",
        "series_manual",
        "saga",
        "available",
        "added_at",
        "tmdb_id",
    ];
    let mut data = row.data.as_object().ok_or_else(invalid)?.clone();
    if data.keys().any(|k| !KEYS.contains(&k.as_str())) {
        return Err(invalid());
    }
    data.insert("id".into(), json!(legacy));
    data.insert(
        "path".into(),
        json!(Path::new(root).join(legacy).to_string_lossy()),
    );
    data.insert("watched".into(), json!(false));
    data.insert("rating".into(), Value::Null);
    let mut m: Media = serde_json::from_value(Value::Object(data)).map_err(|_| invalid())?;
    poster(&m.poster)?;
    MediaPatch {
        title: Some(m.title.clone()),
        category: Some(m.category.clone()),
        year: m.year,
        genres: Some(m.genres.clone()),
        series: Some(m.series.clone().unwrap_or_default()),
        season: Some(m.season),
        episode: Some(m.episode),
        overview: Some(m.overview.clone()),
        runtime_minutes: m.runtime_minutes,
        cast: Some(m.cast.clone()),
        directors: Some(m.directors.clone()),
        series_overview: Some(m.series_overview.clone()),
        ..Default::default()
    }
    .apply(&mut m)?;
    if let Some(info) = &m.series_metadata {
        SeriesPatch {
            name: m.series.clone().ok_or_else(invalid)?,
            year: info.year.unwrap_or(0),
            genres: info.genres.clone(),
            overview: info.overview.clone(),
            cast: info.cast.clone(),
            directors: info.directors.clone(),
        }
        .validate(&m)?;
    }
    if m.added_at < 0 || m.tmdb_id.is_some_and(|id| id <= 0) {
        return Err(invalid());
    }
    m.available = false;
    m.saga = None;
    Ok(m)
}

impl Store {
    pub fn prepare_portable_import(
        &self,
        doc: ArchiveDocument,
        mapping: &ImportMapping,
        root: &Path,
    ) -> Result<PreparedImport, String> {
        doc.validate()?;
        if !relative(&mapping.root_prefix, true)
            || mapping.profile_map.len() != doc.profiles.len()
            || !root.is_absolute()
        {
            return Err("IMPORT_MAPPING_INVALID".into());
        }
        let key = serde_json::to_string(mapping).map_err(|_| invalid())?;
        let already: bool = self
            .personal
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM cc_import_receipts WHERE export_id=?1 AND mapping=?2)",
                params![doc.export_id, key],
                |r| r.get(0),
            )
            .map_err(db)?;
        if already {
            let saved: String = self
                .personal
                .query_row(
                    "SELECT profiles FROM cc_import_receipts WHERE export_id=?1 AND mapping=?2",
                    params![doc.export_id, key],
                    |r| r.get(0),
                )
                .map_err(db)?;
            let profiles = serde_json::from_str(&saved).map_err(|_| invalid())?;
            return Ok(PreparedImport {
                export_id: doc.export_id,
                mapping_key: key,
                epoch: self.catalog_revision()?,
                policy: mapping.conflict_policy,
                profiles,
                media: Vec::new(),
                sagas: Vec::new(),
                personal: Vec::new(),
                conflicts: Vec::new(),
                already_imported: true,
            });
        }
        let mut targets = BTreeSet::new();
        let mut names = BTreeSet::new();
        let mut profiles = Vec::new();
        let current = self.profiles()?;
        for p in &doc.profiles {
            let target = mapping
                .profile_map
                .get(&p.id)
                .ok_or("IMPORT_MAPPING_INVALID")?;
            let (id, name, new) = match target {
                ProfileTarget::Existing {
                    id,
                    expected_revision,
                } => {
                    uuid(id)?;
                    let target = current
                        .iter()
                        .find(|p| p.id == *id && p.revision == *expected_revision)
                        .ok_or("PROFILE_CONFLICT")?;
                    (id.clone(), target.name.clone(), false)
                }
                ProfileTarget::New { name } => {
                    let name = valid_profile_name(name)?;
                    if current.iter().any(|p| p.name.eq_ignore_ascii_case(&name))
                        || !names.insert(name.to_ascii_lowercase())
                    {
                        return Err("IMPORT_MAPPING_INVALID".into());
                    }
                    (uuid::Uuid::new_v4().to_string(), name, true)
                }
            };
            if !targets.insert(id.clone()) {
                return Err("IMPORT_MAPPING_INVALID".into());
            }
            profiles.push(PlannedProfile {
                source: p.id.clone(),
                id,
                name,
                new,
            });
        }
        let mut media = Vec::new();
        let mut media_map = BTreeMap::new();
        let mut conflicts = Vec::new();
        for row in doc.media {
            let legacy = if mapping.root_prefix.is_empty() {
                row.relative_path.clone()
            } else {
                format!("{}/{}", mapping.root_prefix, row.relative_path)
            };
            let existing: Option<String> = self
                .personal
                .query_row(
                    "SELECT stable_id FROM cc_media_identity WHERE relative_path=?1",
                    [&legacy],
                    |r| r.get(0),
                )
                .optional()
                .map_err(db)?;
            let mut id = existing.clone().unwrap_or_else(|| row.id.clone());
            if existing.is_none()
                && self
                    .personal
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM cc_media_identity WHERE stable_id=?1)",
                        [&id],
                        |r| r.get::<_, bool>(0),
                    )
                    .map_err(db)?
            {
                id = uuid::Uuid::new_v4().to_string();
            }
            let mut data = checked_media(&row, &root.to_string_lossy(), &legacy)?;
            let poster_ref = data.poster.clone();
            if existing.is_some() {
                data.available = self
                    .personal
                    .query_row(
                        "SELECT json_extract(data,'$.available') FROM media WHERE id=?1",
                        [&legacy],
                        |r| r.get(0),
                    )
                    .map_err(db)?;
            }
            if existing.is_some() {
                conflicts.push(format!("Fiche : {}", data.title));
            }
            media_map.insert(row.id.clone(), id.clone());
            media.push(PlannedMedia {
                source: row.id,
                id,
                legacy,
                data,
                existing: existing.is_some(),
                poster_ref,
            });
        }
        let mut next_id: i64 = self
            .personal
            .query_row("SELECT COALESCE(MAX(id),0)+1 FROM collections", [], |r| {
                r.get(0)
            })
            .map_err(db)?;
        let mut sagas = Vec::new();
        for s in doc.sagas {
            let exists: Option<i64> = self
                .personal
                .query_row(
                    "SELECT id FROM collections WHERE kind='saga' AND name=?1",
                    [&s.name],
                    |r| r.get(0),
                )
                .optional()
                .map_err(db)?;
            let id = exists.unwrap_or_else(|| {
                let n = next_id;
                next_id += 1;
                n
            });
            if exists.is_some() {
                conflicts.push(format!("Saga : {}", s.name));
            }
            let members = s
                .media_ids
                .iter()
                .map(|id| media_map.get(id).cloned().ok_or_else(invalid))
                .collect::<Result<Vec<_>, _>>()?;
            sagas.push(PlannedSaga {
                id,
                name: s.name,
                members,
                existing: exists.is_some(),
                poster_ref: s.poster,
            });
        }
        let mut personal = doc.personal;
        for p in &mut personal {
            let target = profiles
                .iter()
                .find(|t| t.source == p.profile_id)
                .ok_or_else(invalid)?;
            p.profile_id = target.id.clone();
            if !target.new {
                conflicts.push(format!("Données personnelles : {}", target.name));
            }
            for s in &mut p.states {
                s.media_id = media_map.get(&s.media_id).ok_or_else(invalid)?.clone();
            }
            for l in &mut p.playlists {
                let exists: Option<String> = self
                    .personal
                    .query_row(
                        "SELECT id FROM cc_playlists WHERE profile_id=?1 AND name=?2",
                        params![p.profile_id, l.name],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(db)?;
                l.id = exists.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                for m in &mut l.media_ids {
                    *m = media_map.get(m).ok_or_else(invalid)?.clone();
                }
            }
        }
        Ok(PreparedImport {
            export_id: doc.export_id,
            mapping_key: key,
            epoch: self.catalog_revision()?,
            policy: mapping.conflict_policy,
            profiles,
            media,
            sagas,
            personal,
            conflicts,
            already_imported: already,
        })
    }
    pub fn apply_portable_import(
        &self,
        plan: &PreparedImport,
        poster_paths: &BTreeMap<String, String>,
    ) -> Result<ImportResult, String> {
        let tx =
            rusqlite::Transaction::new_unchecked(&self.personal, TransactionBehavior::Immediate)
                .map_err(db)?;
        let already: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM cc_import_receipts WHERE export_id=?1 AND mapping=?2)",
                params![plan.export_id, plan.mapping_key],
                |r| r.get(0),
            )
            .map_err(db)?;
        if already {
            return Ok(ImportResult {
                already_imported: true,
                media: 0,
                profiles: 0,
                epoch: self.catalog_revision()?,
            });
        }
        let epoch = tx
            .query_row(
                "SELECT revision FROM cc_catalog_epoch WHERE id=1",
                [],
                |r| r.get::<_, i64>(0),
            )
            .map_err(db)? as u64;
        if epoch != plan.epoch {
            return Err("IMPORT_PLAN_STALE".into());
        }
        let replace = plan.policy == ConflictPolicy::ReplaceConflicting;
        for p in &plan.profiles {
            if p.new {
                tx.execute(
                    "INSERT INTO cc_profiles(id,name,revision) VALUES(?1,?2,1)",
                    params![p.id, p.name],
                )
                .map_err(db)?;
            }
        }
        for row in &plan.media {
            if row.existing && !replace {
                continue;
            }
            let mut data = row.data.clone();
            data.poster = match &row.poster_ref {
                Some(reference) => Some(
                    poster_paths
                        .get(reference)
                        .ok_or("IMPORT_POSTER_MISSING")?
                        .clone(),
                ),
                None => None,
            };
            tx.execute("INSERT INTO media(id,data) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET data=excluded.data",params![row.legacy,serde_json::to_string(&data).map_err(|_|invalid())?]).map_err(db)?;
            tx.execute("INSERT OR IGNORE INTO cc_media_identity(stable_id,legacy_id,relative_path) VALUES(?1,?2,?2)",params![row.id,row.legacy]).map_err(db)?;
            tx.execute("INSERT INTO cc_media_revision(stable_id,revision) VALUES(?1,1) ON CONFLICT(stable_id) DO UPDATE SET revision=revision+1",[&row.id]).map_err(db)?;
        }
        for saga in &plan.sagas {
            if saga.existing && !replace {
                continue;
            }
            tx.execute("INSERT INTO collections(id,name,kind) VALUES(?1,?2,'saga') ON CONFLICT(id) DO UPDATE SET name=excluded.name",params![saga.id,saga.name]).map_err(db)?;
            tx.execute(
                "DELETE FROM collection_members WHERE collection_id=?1",
                [saga.id],
            )
            .map_err(db)?;
            tx.execute(
                "DELETE FROM collection_positions WHERE collection_id=?1",
                [saga.id],
            )
            .map_err(db)?;
            for (position, id) in saga.members.iter().enumerate() {
                let legacy: String = tx
                    .query_row(
                        "SELECT legacy_id FROM cc_media_identity WHERE stable_id=?1",
                        [id],
                        |r| r.get(0),
                    )
                    .map_err(db)?;
                let serialized: String = tx
                    .query_row("SELECT data FROM media WHERE id=?1", [&legacy], |r| {
                        r.get(0)
                    })
                    .map_err(db)?;
                let item: Media = serde_json::from_str(&serialized).map_err(|_| invalid())?;
                if item.category != "film" {
                    return Err("IMPORT_SAGA_CATEGORY_CONFLICT".into());
                }
                tx.execute(
                    "INSERT INTO collection_members(collection_id,media_id) VALUES(?1,?2)",
                    params![saga.id, legacy],
                )
                .map_err(db)?;
                tx.execute("INSERT INTO collection_positions(collection_id,media_id,position) VALUES(?1,?2,?3)",params![saga.id,legacy,position as i64]).map_err(db)?;
            }
            tx.execute("INSERT INTO collection_revisions(collection_id,revision) VALUES(?1,1) ON CONFLICT(collection_id) DO UPDATE SET revision=revision+1",[saga.id]).map_err(db)?;
            if let Some(reference) = &saga.poster_ref {
                let path = poster_paths.get(reference).ok_or("IMPORT_POSTER_MISSING")?;
                tx.execute("INSERT INTO collection_artwork(collection_id,poster) VALUES(?1,?2) ON CONFLICT(collection_id) DO UPDATE SET poster=excluded.poster",params![saga.id,path]).map_err(db)?;
            }
        }
        for p in &plan.personal {
            let prefs = &p.preferences;
            if replace {
                tx.execute(
                    "DELETE FROM cc_preferences WHERE profile_id=?1",
                    [&p.profile_id],
                )
                .map_err(db)?;
            }
            tx.execute("INSERT OR IGNORE INTO cc_preferences(profile_id,audio_language,subtitle_language,autoplay_seconds) VALUES(?1,?2,?3,?4)",params![p.profile_id,prefs.audio_language,prefs.subtitle_language,prefs.autoplay_seconds]).map_err(db)?;
            for speed in &p.speeds {
                if replace {
                    tx.execute(
                        "DELETE FROM cc_series_speed WHERE profile_id=?1 AND series_key=?2",
                        params![p.profile_id, speed.series_key],
                    )
                    .map_err(db)?;
                }
                tx.execute("INSERT OR IGNORE INTO cc_series_speed(profile_id,series_key,speed) VALUES(?1,?2,?3)",params![p.profile_id,speed.series_key,speed.speed]).map_err(db)?;
            }
            for state in &p.states {
                if replace {
                    tx.execute(
                        "DELETE FROM cc_profile_state WHERE profile_id=?1 AND stable_media_id=?2",
                        params![p.profile_id, state.media_id],
                    )
                    .map_err(db)?;
                }
                tx.execute("INSERT OR IGNORE INTO cc_profile_state(profile_id,stable_media_id,position_ms,final_watched_ms,watched,rating) VALUES(?1,?2,?3,?4,?5,?6)",params![p.profile_id,state.media_id,state.position_ms as i64,state.final_watched_ms as i64,state.watched,state.rating]).map_err(db)?;
            }
            for list in &p.playlists {
                let exists: bool = tx
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM cc_playlists WHERE id=?1)",
                        [&list.id],
                        |r| r.get(0),
                    )
                    .map_err(db)?;
                if exists && !replace {
                    continue;
                }
                tx.execute("INSERT INTO cc_playlists(id,profile_id,name) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET name=excluded.name,revision=revision+1",params![list.id,p.profile_id,list.name]).map_err(db)?;
                tx.execute(
                    "DELETE FROM cc_playlist_members WHERE playlist_id=?1",
                    [&list.id],
                )
                .map_err(db)?;
                for (position, id) in list.media_ids.iter().enumerate() {
                    tx.execute("INSERT INTO cc_playlist_members(playlist_id,stable_media_id,position) VALUES(?1,?2,?3)",params![list.id,id,position as i64]).map_err(db)?;
                }
            }
        }
        tx.execute(
            "INSERT INTO cc_import_receipts(export_id,mapping,profiles) VALUES(?1,?2,?3)",
            params![
                plan.export_id,
                plan.mapping_key,
                serde_json::to_string(&plan.profiles).map_err(|_| invalid())?
            ],
        )
        .map_err(db)?;
        tx.commit().map_err(db)?;
        Ok(ImportResult {
            already_imported: false,
            media: plan.media.len(),
            profiles: plan.profiles.len(),
            epoch: self.catalog_revision()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn document() -> ArchiveDocument {
        let p = uuid::Uuid::new_v4().to_string();
        let a = uuid::Uuid::new_v4().to_string();
        let b = uuid::Uuid::new_v4().to_string();
        let data = |title: &str| json!({"id":format!("{title}.mkv"),"title":title,"category":"film","year":2026,"series":null,"season":null,"episode":null,"genres":[],"overview":"Résumé","poster":null,"available":true,"added_at":1,"tmdb_id":null});
        ArchiveDocument {
            format_version: 1,
            export_id: uuid::Uuid::new_v4().to_string(),
            profiles: vec![crate::Profile {
                id: p.clone(),
                name: "Source".into(),
                revision: 1,
            }],
            media: vec![
                PortableMedia {
                    id: a.clone(),
                    relative_path: "Films/A.mkv".into(),
                    data: data("A"),
                },
                PortableMedia {
                    id: b.clone(),
                    relative_path: "Films/B.mkv".into(),
                    data: data("B"),
                },
            ],
            sagas: vec![Collection {
                id: "1".into(),
                name: "Saga".into(),
                kind: "saga".into(),
                media_ids: vec![b.clone(), a.clone()],
                poster: None,
            }],
            personal: vec![PortablePersonal {
                profile_id: p,
                preferences: Preferences::default(),
                speeds: vec![PortableSpeed {
                    series_key: "Série future".into(),
                    speed: 1.5,
                }],
                states: vec![PortableState {
                    media_id: a.clone(),
                    position_ms: 1234,
                    final_watched_ms: 10000,
                    watched: true,
                    rating: Some(4),
                }],
                playlists: vec![PortablePlaylist {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: "À regarder".into(),
                    media_ids: vec![a, b],
                }],
            }],
            leases: Vec::new(),
        }
    }
    fn mapping(d: &ArchiveDocument) -> ImportMapping {
        ImportMapping {
            profile_map: BTreeMap::from([(
                d.profiles[0].id.clone(),
                ProfileTarget::New {
                    name: "Destination".into(),
                },
            )]),
            root_prefix: String::new(),
            conflict_policy: ConflictPolicy::KeepDestination,
        }
    }
    #[test]
    fn new_profile_roundtrip_is_readonly_until_apply_atomic_and_idempotent() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db.sqlite")).unwrap();
        let d = document();
        let map = mapping(&d);
        let plan = store
            .prepare_portable_import(d.clone(), &map, Path::new("/srv/videos"))
            .unwrap();
        assert!(store.profiles().unwrap().is_empty());
        assert_eq!(store.catalog_revision().unwrap(), plan.epoch);
        assert!(
            !store
                .apply_portable_import(&plan, &BTreeMap::new())
                .unwrap()
                .already_imported
        );
        let profile = &plan.profiles[0].id;
        let list = store.playlists(profile).unwrap();
        assert_eq!(
            list[0].media_ids,
            plan.media.iter().map(|m| m.id.clone()).collect::<Vec<_>>()
        );
        assert_eq!(
            store.sagas().unwrap()[0].media_ids,
            vec![plan.media[1].id.clone(), plan.media[0].id.clone()]
        );
        assert_eq!(
            store
                .personal_state(profile, &plan.media[0].id)
                .unwrap()
                .rating,
            Some(4)
        );
        assert_eq!(store.series_speed(profile, "Série future").unwrap(), 1.5);
        assert!(!store.media(profile, &plan.media[0].id).unwrap().available);
        let again = store
            .prepare_portable_import(d, &map, Path::new("/srv/videos"))
            .unwrap();
        assert!(again.already_imported);
        assert_eq!(again.profiles[0].id, *profile);
        assert!(
            store
                .apply_portable_import(&again, &BTreeMap::new())
                .unwrap()
                .already_imported
        );
        assert_eq!(store.profiles().unwrap().len(), 1);
    }
    #[test]
    fn bad_references_mapping_stale_and_sql_failure_never_partially_import() {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::open(&temp.path().join("db.sqlite")).unwrap();
        let mut d = document();
        let map = mapping(&d);
        d.personal[0].states[0].media_id = uuid::Uuid::new_v4().to_string();
        assert!(
            store
                .prepare_portable_import(d, &map, Path::new("/srv/videos"))
                .is_err()
        );
        assert!(store.profiles().unwrap().is_empty());
        let d = document();
        let plan = store
            .prepare_portable_import(d.clone(), &mapping(&d), Path::new("/srv/videos"))
            .unwrap();
        store.create_profile("Unrelated change").unwrap();
        assert_eq!(
            store
                .apply_portable_import(&plan, &BTreeMap::new())
                .unwrap_err(),
            "IMPORT_PLAN_STALE"
        );
        assert_eq!(store.profiles().unwrap().len(), 1);
        let fresh = store
            .prepare_portable_import(d.clone(), &mapping(&d), Path::new("/srv/videos"))
            .unwrap();
        store.personal.execute_batch("CREATE TRIGGER fail_import BEFORE INSERT ON cc_playlist_members BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
        let epoch = store.catalog_revision().unwrap();
        assert!(
            store
                .apply_portable_import(&fresh, &BTreeMap::new())
                .is_err()
        );
        assert_eq!(store.catalog_revision().unwrap(), epoch);
        assert_eq!(store.profiles().unwrap().len(), 1);
        assert_eq!(
            store
                .personal
                .query_row("SELECT COUNT(*) FROM media", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    #[test]
    fn explicit_profile_mapping_is_injective_and_preserves_other_profiles() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db.sqlite")).unwrap();
        let a = store.create_profile("A").unwrap();
        let b = store.create_profile("B").unwrap();
        let mut d = document();
        let second = crate::Profile {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Source 2".into(),
            revision: 1,
        };
        let mut state = d.personal[0].clone();
        state.profile_id = second.id.clone();
        state.states[0].rating = Some(2);
        d.personal.push(state);
        d.profiles.push(second);
        let mut map = ImportMapping {
            profile_map: BTreeMap::new(),
            root_prefix: "Migration".into(),
            conflict_policy: ConflictPolicy::ReplaceConflicting,
        };
        for p in &d.profiles {
            map.profile_map.insert(
                p.id.clone(),
                ProfileTarget::Existing {
                    id: a.id.clone(),
                    expected_revision: 1,
                },
            );
        }
        assert!(
            store
                .prepare_portable_import(d.clone(), &map, Path::new("/srv/videos"))
                .is_err()
        );
        map.profile_map.insert(
            d.profiles[1].id.clone(),
            ProfileTarget::Existing {
                id: b.id.clone(),
                expected_revision: 1,
            },
        );
        let plan = store
            .prepare_portable_import(d, &map, Path::new("/srv/videos"))
            .unwrap();
        store
            .apply_portable_import(&plan, &BTreeMap::new())
            .unwrap();
        assert_eq!(
            store
                .personal_state(&a.id, &plan.media[0].id)
                .unwrap()
                .rating,
            Some(4)
        );
        assert_eq!(
            store
                .personal_state(&b.id, &plan.media[0].id)
                .unwrap()
                .rating,
            Some(2)
        );
        assert_eq!(store.playlists(&a.id).unwrap().len(), 1);
        assert_eq!(store.playlists(&b.id).unwrap().len(), 1);
        assert_eq!(
            store.media(&a.id, &plan.media[0].id).unwrap().path,
            "Migration/Films/A.mkv"
        );
    }
}
