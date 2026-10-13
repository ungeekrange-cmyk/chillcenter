//! Profile-scoped persistence over the legacy common catalogue.
//!
//! The catalogue continues to own provider metadata and physical locations.  This
//! module deliberately never opens or walks a media path: `media_path` is the
//! narrow server-side escape hatch and all catalogue views expose relative paths.

use crate::{PersonalState, Preferences, Profile, valid_speed};
use chill_catalog::{
    database::Database,
    model::{Collection, Media, MediaPatch},
};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};
use uuid::Uuid;

const EXPORT_FORMAT_VERSION: u64 = 1;
const WATCHED_THRESHOLD_MS: u64 = 10_000;

pub struct Store {
    catalogue: Database,
    personal: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self, String> {
        let catalogue = Database::open(path)?;
        let personal = Connection::open(path).map_err(db_error)?;
        personal
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(db_error)?;
        personal.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS cc_profiles (
                id TEXT PRIMARY KEY, name TEXT NOT NULL COLLATE NOCASE UNIQUE,
                revision INTEGER NOT NULL DEFAULT 1
             );
             CREATE TABLE IF NOT EXISTS cc_media_identity (
                stable_id TEXT PRIMARY KEY, legacy_id TEXT NOT NULL UNIQUE,
                relative_path TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS cc_profile_state (
                profile_id TEXT NOT NULL REFERENCES cc_profiles(id) ON DELETE CASCADE,
                stable_media_id TEXT NOT NULL REFERENCES cc_media_identity(stable_id),
                position_ms INTEGER NOT NULL DEFAULT 0 CHECK(position_ms >= 0),
                final_watched_ms INTEGER NOT NULL DEFAULT 0 CHECK(final_watched_ms >= 0),
                watched INTEGER NOT NULL DEFAULT 0 CHECK(watched IN (0,1)),
                rating INTEGER CHECK(rating BETWEEN 1 AND 5),
                PRIMARY KEY(profile_id, stable_media_id)
             );
             CREATE TABLE IF NOT EXISTS cc_preferences (
                profile_id TEXT PRIMARY KEY REFERENCES cc_profiles(id) ON DELETE CASCADE,
                audio_language TEXT NOT NULL, subtitle_language TEXT NOT NULL,
                autoplay_seconds INTEGER
             );
             CREATE TABLE IF NOT EXISTS cc_series_speed (
                profile_id TEXT NOT NULL REFERENCES cc_profiles(id) ON DELETE CASCADE,
                series_key TEXT NOT NULL, speed REAL NOT NULL,
                PRIMARY KEY(profile_id, series_key)
             );
             CREATE TABLE IF NOT EXISTS cc_playlists (
                id TEXT PRIMARY KEY, profile_id TEXT NOT NULL REFERENCES cc_profiles(id) ON DELETE CASCADE,
                name TEXT NOT NULL COLLATE NOCASE, revision INTEGER NOT NULL DEFAULT 1,
                UNIQUE(profile_id, name)
             );
             CREATE TABLE IF NOT EXISTS cc_playlist_members (
                playlist_id TEXT NOT NULL REFERENCES cc_playlists(id) ON DELETE CASCADE,
                stable_media_id TEXT NOT NULL REFERENCES cc_media_identity(stable_id),
                position INTEGER NOT NULL CHECK(position >= 0),
                PRIMARY KEY(playlist_id, stable_media_id), UNIQUE(playlist_id, position)
             );
             CREATE TABLE IF NOT EXISTS cc_imports (export_id TEXT PRIMARY KEY);
             PRAGMA user_version=2;"
        ).map_err(db_error)?;
        Ok(Self {
            catalogue,
            personal,
        })
    }

    pub fn profiles(&self) -> Result<Vec<Profile>, String> {
        let mut statement = self
            .personal
            .prepare("SELECT id,name,revision FROM cc_profiles ORDER BY name,id")
            .map_err(db_error)?;
        statement
            .query_map([], |row| {
                Ok(Profile {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    revision: row.get::<_, i64>(2)? as u64,
                })
            })
            .map_err(db_error)?
            .collect::<Result<_, _>>()
            .map_err(db_error)
    }

    pub fn create_profile(&self, name: &str) -> Result<Profile, String> {
        let name = valid_profile_name(name)?;
        let profile = Profile {
            id: Uuid::new_v4().to_string(),
            name,
            revision: 1,
        };
        self.personal
            .execute(
                "INSERT INTO cc_profiles(id,name,revision) VALUES(?1,?2,1)",
                params![profile.id, profile.name],
            )
            .map_err(profile_collision)?;
        Ok(profile)
    }

    pub fn rename_profile(&self, id: &str, name: &str) -> Result<Profile, String> {
        let name = valid_profile_name(name)?;
        if self
            .personal
            .execute(
                "UPDATE cc_profiles SET name=?1,revision=revision+1 WHERE id=?2",
                params![name, id],
            )
            .map_err(profile_collision)?
            != 1
        {
            return Err("Profil introuvable.".into());
        }
        self.profile(id)?
            .ok_or_else(|| "Profil introuvable.".into())
    }

    /// Deletes profile-scoped rows through FKs. It never touches common catalogue rows or files.
    pub fn delete_profile(&self, id: &str) -> Result<(), String> {
        if self
            .personal
            .execute("DELETE FROM cc_profiles WHERE id=?1", [id])
            .map_err(db_error)?
            != 1
        {
            return Err("Profil introuvable.".into());
        }
        Ok(())
    }

    pub fn preferences(&self, profile_id: &str) -> Result<Preferences, String> {
        self.require_profile(profile_id)?;
        self.personal.query_row("SELECT audio_language,subtitle_language,autoplay_seconds FROM cc_preferences WHERE profile_id=?1", [profile_id], |r| Ok(Preferences { audio_language: r.get(0)?, subtitle_language: r.get(1)?, autoplay_seconds: r.get::<_, Option<i64>>(2)?.map(|v| v as u8) }))
            .optional().map_err(db_error)?.map(Ok).unwrap_or_else(|| Ok(Preferences::default()))
    }

    pub fn save_preferences(
        &self,
        profile_id: &str,
        preferences: &Preferences,
    ) -> Result<(), String> {
        self.require_profile(profile_id)?;
        preferences.validate()?;
        self.personal.execute("INSERT INTO cc_preferences(profile_id,audio_language,subtitle_language,autoplay_seconds) VALUES(?1,?2,?3,?4) ON CONFLICT(profile_id) DO UPDATE SET audio_language=excluded.audio_language,subtitle_language=excluded.subtitle_language,autoplay_seconds=excluded.autoplay_seconds", params![profile_id, preferences.audio_language, preferences.subtitle_language, preferences.autoplay_seconds]).map_err(db_error)?;
        Ok(())
    }

    pub fn set_series_speed(
        &self,
        profile_id: &str,
        series_key: &str,
        speed: f64,
    ) -> Result<(), String> {
        self.require_profile(profile_id)?;
        if series_key.trim().is_empty() || series_key.len() > 500 || !valid_speed(speed) {
            return Err("Série ou vitesse invalide.".into());
        }
        self.personal.execute("INSERT INTO cc_series_speed(profile_id,series_key,speed) VALUES(?1,?2,?3) ON CONFLICT(profile_id,series_key) DO UPDATE SET speed=excluded.speed", params![profile_id, series_key.trim(), speed]).map_err(db_error)?;
        Ok(())
    }

    pub fn series_speed(&self, profile_id: &str, series_key: &str) -> Result<f64, String> {
        self.require_profile(profile_id)?;
        Ok(self
            .personal
            .query_row(
                "SELECT speed FROM cc_series_speed WHERE profile_id=?1 AND series_key=?2",
                params![profile_id, series_key],
                |r| r.get(0),
            )
            .optional()
            .map_err(db_error)?
            .unwrap_or(1.0))
    }

    pub fn catalog(&mut self, profile_id: &str) -> Result<Vec<Media>, String> {
        self.require_profile(profile_id)?;
        let media = self.catalogue.list()?;
        self.ensure_identities(&media)?;
        media
            .into_iter()
            .map(|m| self.profile_view(profile_id, m))
            .collect()
    }

    pub fn media(&mut self, profile_id: &str, stable_id: &str) -> Result<Media, String> {
        self.require_profile(profile_id)?;
        let legacy = self.legacy_id(stable_id)?;
        self.profile_view(profile_id, self.catalogue.get(&legacy)?)
    }

    /// Absolute paths stay within the server process and are never serialized in catalogue views.
    pub fn media_path(&self, stable_id: &str) -> Result<PathBuf, String> {
        let legacy = self.legacy_id(stable_id)?;
        Ok(PathBuf::from(self.catalogue.get(&legacy)?.path))
    }

    pub fn snapshot(&mut self, discovered: Vec<Media>) -> Result<i64, String> {
        // Legacy snapshot is all-or-nothing and specifically preserves a prior catalogue on an empty scan.
        let scan = self.catalogue.snapshot(discovered)?;
        self.ensure_identities(&self.catalogue.list()?)?;
        Ok(scan)
    }

    pub fn update_media(
        &mut self,
        profile_id: &str,
        stable_id: &str,
        patch: &MediaPatch,
    ) -> Result<Media, String> {
        self.require_profile(profile_id)?;
        if patch.rating.is_some() || patch.watched.is_some() {
            return Err("La note et le vu sont propres au profil.".into());
        }
        let legacy = self.legacy_id(stable_id)?;
        let updated = self.catalogue.patch(&legacy, patch)?;
        self.profile_view(profile_id, updated)
    }

    pub fn set_rating(
        &mut self,
        profile_id: &str,
        stable_id: &str,
        rating: Option<u8>,
        scope: &str,
    ) -> Result<(), String> {
        self.require_profile(profile_id)?;
        if rating.is_some_and(|v| !(1..=5).contains(&v))
            || !["video", "season", "series"].contains(&scope)
        {
            return Err("Note ou portée invalide.".into());
        }
        let target = self.media(profile_id, stable_id)?;
        let targets = if scope == "video" {
            vec![target.id]
        } else {
            if target.category != "serie" {
                return Err("Cette portée est réservée aux séries.".into());
            }
            self.catalog(profile_id)?
                .into_iter()
                .filter(|m| {
                    m.category == "serie"
                        && m.series == target.series
                        && (scope == "series" || m.season == target.season)
                })
                .map(|m| m.id)
                .collect()
        };
        let tx = self.personal.unchecked_transaction().map_err(db_error)?;
        for id in targets {
            upsert_state(&tx, profile_id, &id, None, None, None, Some(rating))?;
        }
        tx.commit().map_err(db_error)
    }

    pub fn set_watched(
        &self,
        profile_id: &str,
        stable_id: &str,
        watched: bool,
    ) -> Result<(), String> {
        self.require_profile(profile_id)?;
        self.legacy_id(stable_id)?;
        let tx = self.personal.unchecked_transaction().map_err(db_error)?;
        // Manual false deliberately resets only the cumulative watched counter, not resume position.
        upsert_state(
            &tx,
            profile_id,
            stable_id,
            None,
            Some(if watched { None } else { Some(0) }),
            Some(watched),
            None,
        )?;
        tx.commit().map_err(db_error)
    }

    pub fn save_progress(
        &self,
        profile_id: &str,
        stable_id: &str,
        position_ms: u64,
        final_elapsed_ms: u64,
        duration_ms: u64,
    ) -> Result<PersonalState, String> {
        self.require_profile(profile_id)?;
        self.legacy_id(stable_id)?;
        if duration_ms == 0
            || position_ms > duration_ms
            || final_elapsed_ms > duration_ms.saturating_add(WATCHED_THRESHOLD_MS)
        {
            return Err("Progression invalide.".into());
        }
        let tx = self.personal.unchecked_transaction().map_err(db_error)?;
        let old = state_in(&tx, profile_id, stable_id)?;
        let final_watched_ms = old.final_watched_ms.saturating_add(final_elapsed_ms);
        let watched = old.watched || final_watched_ms >= WATCHED_THRESHOLD_MS;
        upsert_state(
            &tx,
            profile_id,
            stable_id,
            Some(position_ms),
            Some(Some(final_watched_ms)),
            Some(watched),
            None,
        )?;
        tx.commit().map_err(db_error)?;
        Ok(PersonalState {
            position_ms,
            final_watched_ms,
            watched,
            rating: old.rating,
        })
    }

    pub fn collections(&mut self, profile_id: &str) -> Result<Vec<Collection>, String> {
        self.require_profile(profile_id)?;
        let mut collections = self.catalogue.collections()?;
        self.ensure_identities(&self.catalogue.list()?)?;
        for collection in &mut collections {
            collection.media_ids = collection
                .media_ids
                .iter()
                .map(|legacy_id| self.stable_id(legacy_id))
                .collect::<Result<_, _>>()?;
        }
        let mut stmt = self
            .personal
            .prepare("SELECT id,name FROM cc_playlists WHERE profile_id=?1 ORDER BY name,id")
            .map_err(db_error)?;
        let rows = stmt
            .query_map([profile_id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(db_error)?;
        for row in rows {
            let (id, name) = row.map_err(db_error)?;
            let media_ids = self.playlist_members(&id)?;
            collections.push(Collection {
                id,
                name,
                kind: "playlist".into(),
                media_ids,
                poster: None,
            });
        }
        Ok(collections)
    }

    pub fn create_collection(
        &self,
        profile_id: &str,
        name: &str,
        kind: &str,
    ) -> Result<Collection, String> {
        self.require_profile(profile_id)?;
        if kind != "playlist" {
            return Err("Seules les playlists personnelles peuvent être créées ici.".into());
        }
        let name = valid_collection_name(name)?;
        let id = Uuid::new_v4().to_string();
        self.personal
            .execute(
                "INSERT INTO cc_playlists(id,profile_id,name) VALUES(?1,?2,?3)",
                params![id, profile_id, name],
            )
            .map_err(|_| "Une playlist porte déjà ce nom pour ce profil.".to_string())?;
        Ok(Collection {
            id,
            name,
            kind: kind.into(),
            media_ids: vec![],
            poster: None,
        })
    }

    pub fn collection_member_add_bulk(
        &self,
        profile_id: &str,
        collection_id: &str,
        media_ids: &[String],
    ) -> Result<(), String> {
        self.require_playlist(profile_id, collection_id)?;
        if media_ids.is_empty() || media_ids.len() > 1000 {
            return Err("Sélectionnez de 1 à 1 000 vidéos par ajout.".into());
        }
        let mut seen = HashSet::new();
        if media_ids.iter().any(|id| !seen.insert(id)) {
            return Err("Sélection invalide : vidéo dupliquée.".into());
        }
        let tx = self.personal.unchecked_transaction().map_err(db_error)?;
        let start: i64 = tx
            .query_row(
                "SELECT COALESCE(MAX(position)+1,0) FROM cc_playlist_members WHERE playlist_id=?1",
                [collection_id],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        for (index, id) in media_ids.iter().enumerate() {
            ensure_identity_in(&tx, id)?;
            tx.execute("INSERT OR IGNORE INTO cc_playlist_members(playlist_id,stable_media_id,position) VALUES(?1,?2,?3)", params![collection_id,id,start + index as i64]).map_err(db_error)?;
        }
        tx.execute(
            "UPDATE cc_playlists SET revision=revision+1 WHERE id=?1",
            [collection_id],
        )
        .map_err(db_error)?;
        tx.commit().map_err(db_error)
    }

    pub fn collection_member_remove(
        &self,
        profile_id: &str,
        collection_id: &str,
        stable_id: &str,
    ) -> Result<(), String> {
        self.require_playlist(profile_id, collection_id)?;
        self.legacy_id(stable_id)?;
        self.personal
            .execute(
                "DELETE FROM cc_playlist_members WHERE playlist_id=?1 AND stable_media_id=?2",
                params![collection_id, stable_id],
            )
            .map_err(db_error)?;
        self.normalize_playlist(collection_id)
    }

    pub fn reorder_playlist(
        &self,
        profile_id: &str,
        collection_id: &str,
        expected_order: &[String],
        desired_order: &[String],
    ) -> Result<(), String> {
        self.require_playlist(profile_id, collection_id)?;
        let tx = self.personal.unchecked_transaction().map_err(db_error)?;
        let current = playlist_members_in(&tx, collection_id)?;
        if current != expected_order {
            return Err(
                "La liste a changé entre-temps. Actualisez-la avant de la réorganiser.".into(),
            );
        }
        let desired: HashSet<_> = desired_order.iter().collect();
        if desired_order.len() != current.len()
            || desired.len() != current.len()
            || !current.iter().all(|id| desired.contains(id))
        {
            return Err(
                "Ordre invalide : conservez chaque vidéo de la liste exactement une fois.".into(),
            );
        }
        // Move out of the unique position range first, so a swap cannot fail
        // halfway through under SQLite's immediate uniqueness checks.
        let temporary_offset = current.len() as i64 + 1;
        for (position, id) in current.iter().enumerate() {
            tx.execute(
                "UPDATE cc_playlist_members SET position=?1 WHERE playlist_id=?2 AND stable_media_id=?3",
                params![temporary_offset + position as i64, collection_id, id],
            )
            .map_err(db_error)?;
        }
        for (position, id) in desired_order.iter().enumerate() {
            tx.execute("UPDATE cc_playlist_members SET position=?1 WHERE playlist_id=?2 AND stable_media_id=?3", params![position as i64,collection_id,id]).map_err(db_error)?;
        }
        tx.execute(
            "UPDATE cc_playlists SET revision=revision+1 WHERE id=?1",
            [collection_id],
        )
        .map_err(db_error)?;
        tx.commit().map_err(db_error)
    }

    pub fn rename_collection(
        &self,
        profile_id: &str,
        collection_id: &str,
        name: &str,
    ) -> Result<(), String> {
        self.require_playlist(profile_id, collection_id)?;
        let name = valid_collection_name(name)?;
        self.personal
            .execute(
                "UPDATE cc_playlists SET name=?1,revision=revision+1 WHERE id=?2",
                params![name, collection_id],
            )
            .map_err(|_| "Une playlist porte déjà ce nom pour ce profil.".to_string())?;
        Ok(())
    }
    pub fn delete_collection(&self, profile_id: &str, collection_id: &str) -> Result<(), String> {
        self.require_playlist(profile_id, collection_id)?;
        self.personal
            .execute("DELETE FROM cc_playlists WHERE id=?1", [collection_id])
            .map_err(db_error)?;
        Ok(())
    }

    pub fn export_json(&mut self) -> Result<Value, String> {
        let profiles = self.profiles()?;
        let media = self.catalogue.list()?;
        self.ensure_identities(&media)?;
        let identities: Vec<Value> = media.iter().map(|m| Ok(json!({"id": self.stable_id(&m.id)?, "relative_path": m.id, "data": scrub_media(m)}))).collect::<Result<_, String>>()?;
        let personal = profiles
            .iter()
            .map(|p| self.export_profile(p))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(
            json!({"format_version": EXPORT_FORMAT_VERSION, "export_id": Uuid::new_v4().to_string(), "profiles": profiles, "media": identities, "personal": personal, "leases": []}),
        )
    }

    /// Cheap structural preview. Applying imported records is intentionally left to the
    /// admin transaction/coordinator; this avoids an implicit destructive restore.
    pub fn preview_import(&self, document: &Value) -> Result<(String, usize, usize), String> {
        if document.get("format_version").and_then(Value::as_u64) != Some(EXPORT_FORMAT_VERSION) {
            return Err("Format d’import inconnu.".into());
        }
        let export_id = document
            .get("export_id")
            .and_then(Value::as_str)
            .filter(|v| Uuid::parse_str(v).is_ok())
            .ok_or("Identifiant d’export invalide.")?;
        let profiles = document
            .get("profiles")
            .and_then(Value::as_array)
            .ok_or("Profils d’import invalides.")?
            .len();
        let media = document
            .get("media")
            .and_then(Value::as_array)
            .ok_or("Médias d’import invalides.")?
            .len();
        Ok((export_id.to_owned(), profiles, media))
    }

    /// Applies only data whose profile and relative-location mappings were explicitly
    /// prepared by the administrator. It deliberately does not restore leases or
    /// infer a NAS root. Repeating an `export_id` is a no-op.
    pub fn import_prepared_json(
        &mut self,
        document: &Value,
        profile_map: &HashMap<String, String>,
    ) -> Result<bool, String> {
        let (export_id, _, _) = self.preview_import(document)?;
        if self
            .personal
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM cc_imports WHERE export_id=?1)",
                [&export_id],
                |r| r.get(0),
            )
            .map_err(db_error)?
        {
            return Ok(false);
        }
        self.ensure_identities(&self.catalogue.list()?)?;
        let mut imported_media = HashMap::new();
        for item in document["media"]
            .as_array()
            .ok_or("Médias d’import invalides.")?
        {
            let old_id = item["id"].as_str().ok_or("Média d’import invalide.")?;
            let relative = item["relative_path"]
                .as_str()
                .ok_or("Localisation d’import invalide.")?;
            let local: Option<String> = self
                .personal
                .query_row(
                    "SELECT stable_id FROM cc_media_identity WHERE relative_path=?1",
                    [relative],
                    |r| r.get(0),
                )
                .optional()
                .map_err(db_error)?;
            if let Some(local) = local {
                imported_media.insert(old_id.to_owned(), local);
            }
        }
        let personal = document["personal"]
            .as_array()
            .ok_or("Données personnelles d’import invalides.")?;
        // Validate all mappings before opening the write transaction.
        for entry in personal {
            let source_profile = entry["profile_id"]
                .as_str()
                .ok_or("Profil d’import invalide.")?;
            let destination = profile_map
                .get(source_profile)
                .ok_or("Correspondance de profil requise.")?;
            self.require_profile(destination)?;
            for state in entry["states"]
                .as_array()
                .ok_or("États d’import invalides.")?
            {
                let old_media = state["media_id"]
                    .as_str()
                    .ok_or("État d’import invalide.")?;
                if !imported_media.contains_key(old_media) {
                    return Err("Correspondance de racine ou média requise.".into());
                }
            }
        }
        let tx = self.personal.unchecked_transaction().map_err(db_error)?;
        for entry in personal {
            let source_profile = entry["profile_id"].as_str().unwrap();
            let destination = profile_map.get(source_profile).unwrap();
            let preferences: Preferences = serde_json::from_value(entry["preferences"].clone())
                .map_err(|_| "Préférences d’import invalides.")?;
            preferences.validate()?;
            tx.execute("INSERT INTO cc_preferences(profile_id,audio_language,subtitle_language,autoplay_seconds) VALUES(?1,?2,?3,?4) ON CONFLICT(profile_id) DO UPDATE SET audio_language=excluded.audio_language,subtitle_language=excluded.subtitle_language,autoplay_seconds=excluded.autoplay_seconds", params![destination,preferences.audio_language,preferences.subtitle_language,preferences.autoplay_seconds]).map_err(db_error)?;
            for state in entry["states"].as_array().unwrap() {
                let local_media = imported_media
                    .get(state["media_id"].as_str().unwrap())
                    .unwrap();
                let position = state["position_ms"]
                    .as_u64()
                    .ok_or("État d’import invalide.")?;
                let final_ms = state["final_watched_ms"]
                    .as_u64()
                    .ok_or("État d’import invalide.")?;
                let watched = state["watched"]
                    .as_bool()
                    .ok_or("État d’import invalide.")?;
                let rating = state["rating"].as_u64().map(|v| v as u8);
                if rating.is_some_and(|v| !(1..=5).contains(&v)) {
                    return Err("Note d’import invalide.".into());
                }
                tx.execute("INSERT INTO cc_profile_state(profile_id,stable_media_id,position_ms,final_watched_ms,watched,rating) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(profile_id,stable_media_id) DO UPDATE SET position_ms=excluded.position_ms,final_watched_ms=excluded.final_watched_ms,watched=excluded.watched,rating=excluded.rating", params![destination,local_media,position as i64,final_ms as i64,watched,rating]).map_err(db_error)?;
            }
            for playlist in entry["playlists"]
                .as_array()
                .ok_or("Playlists d’import invalides.")?
            {
                let name = valid_collection_name(
                    playlist["name"]
                        .as_str()
                        .ok_or("Playlist d’import invalide.")?,
                )?;
                let id = Uuid::new_v4().to_string();
                tx.execute(
                    "INSERT INTO cc_playlists(id,profile_id,name) VALUES(?1,?2,?3)",
                    params![id, destination, name],
                )
                .map_err(|_| {
                    "Conflit de playlist : choisissez une correspondance avant import.".to_string()
                })?;
                let members = playlist["media_ids"]
                    .as_array()
                    .ok_or("Membres d’import invalides.")?;
                let mut seen = HashSet::new();
                for (position, old_media) in members.iter().enumerate() {
                    let old_media = old_media.as_str().ok_or("Membre d’import invalide.")?;
                    let local = imported_media
                        .get(old_media)
                        .ok_or("Correspondance de racine ou média requise.")?;
                    if !seen.insert(local) {
                        return Err("Playlist d’import dupliquée.".into());
                    }
                    tx.execute("INSERT INTO cc_playlist_members(playlist_id,stable_media_id,position) VALUES(?1,?2,?3)",params![id,local,position as i64]).map_err(db_error)?;
                }
            }
        }
        tx.execute("INSERT INTO cc_imports(export_id) VALUES(?1)", [&export_id])
            .map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        Ok(true)
    }

    fn profile(&self, id: &str) -> Result<Option<Profile>, String> {
        self.personal
            .query_row(
                "SELECT id,name,revision FROM cc_profiles WHERE id=?1",
                [id],
                |r| {
                    Ok(Profile {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        revision: r.get::<_, i64>(2)? as u64,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }
    fn require_profile(&self, id: &str) -> Result<(), String> {
        self.profile(id)?
            .map(|_| ())
            .ok_or_else(|| "Profil introuvable.".into())
    }
    fn require_playlist(&self, profile_id: &str, id: &str) -> Result<(), String> {
        let exists: bool = self
            .personal
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM cc_playlists WHERE id=?1 AND profile_id=?2)",
                params![id, profile_id],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        exists
            .then_some(())
            .ok_or_else(|| "Playlist introuvable.".into())
    }
    fn ensure_identities(&self, media: &[Media]) -> Result<(), String> {
        let tx = self.personal.unchecked_transaction().map_err(db_error)?;
        for m in media {
            tx.execute("INSERT OR IGNORE INTO cc_media_identity(stable_id,legacy_id,relative_path) VALUES(?1,?2,?3)",params![Uuid::new_v4().to_string(),m.id,m.id]).map_err(db_error)?;
        }
        tx.commit().map_err(db_error)
    }
    fn stable_id(&self, legacy: &str) -> Result<String, String> {
        self.personal
            .query_row(
                "SELECT stable_id FROM cc_media_identity WHERE legacy_id=?1",
                [legacy],
                |r| r.get(0),
            )
            .map_err(|_| "Vidéo introuvable dans le catalogue.".into())
    }
    fn legacy_id(&self, stable: &str) -> Result<String, String> {
        self.personal
            .query_row(
                "SELECT legacy_id FROM cc_media_identity WHERE stable_id=?1",
                [stable],
                |r| r.get(0),
            )
            .map_err(|_| "Vidéo introuvable dans le catalogue.".into())
    }
    fn profile_view(&self, profile: &str, mut media: Media) -> Result<Media, String> {
        let stable = self.stable_id(&media.id)?;
        let state = state(&self.personal, profile, &stable)?;
        media.id = stable;
        media.path = self
            .personal
            .query_row(
                "SELECT relative_path FROM cc_media_identity WHERE stable_id=?1",
                [&media.id],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        media.watched = state.watched;
        media.rating = state.rating;
        Ok(media)
    }
    fn playlist_members(&self, id: &str) -> Result<Vec<String>, String> {
        playlist_members(&self.personal, id)
    }
    fn normalize_playlist(&self, id: &str) -> Result<(), String> {
        let tx = self.personal.unchecked_transaction().map_err(db_error)?;
        for (index, media) in playlist_members_in(&tx, id)?.iter().enumerate() {
            tx.execute("UPDATE cc_playlist_members SET position=?1 WHERE playlist_id=?2 AND stable_media_id=?3",params![index as i64,id,media]).map_err(db_error)?;
        }
        tx.commit().map_err(db_error)
    }
    fn export_profile(&self, p: &Profile) -> Result<Value, String> {
        let prefs = self.preferences(&p.id)?;
        let mut states = Vec::new();
        let mut q=self.personal.prepare("SELECT stable_media_id,position_ms,final_watched_ms,watched,rating FROM cc_profile_state WHERE profile_id=?1").map_err(db_error)?;
        for r in q.query_map([&p.id],|r|Ok(json!({"media_id":r.get::<_,String>(0)?,"position_ms":r.get::<_,i64>(1)?,"final_watched_ms":r.get::<_,i64>(2)?,"watched":r.get::<_,bool>(3)?,"rating":r.get::<_,Option<i64>>(4)?}))).map_err(db_error)? { states.push(r.map_err(db_error)?); }
        Ok(
            json!({"profile_id":p.id,"preferences":prefs,"states":states,"playlists":self.export_playlists(&p.id)?}),
        )
    }
    fn export_playlists(&self, profile: &str) -> Result<Vec<Value>, String> {
        let mut q = self
            .personal
            .prepare("SELECT id,name FROM cc_playlists WHERE profile_id=?1 ORDER BY id")
            .map_err(db_error)?;
        q.query_map([profile], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(db_error)?
        .map(|r| {
            let (id, name) = r.map_err(db_error)?;
            Ok(json!({"id":id,"name":name,"media_ids":self.playlist_members(&id)?}))
        })
        .collect()
    }
}

fn db_error(_: rusqlite::Error) -> String {
    "Erreur de base locale. Vos données n’ont pas été remplacées.".into()
}
fn profile_collision(_: rusqlite::Error) -> String {
    "Ce nom de profil est déjà utilisé.".into()
}
fn valid_profile_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() || name.len() > 80 || name.chars().any(|c| c.is_control()) {
        Err("Nom de profil invalide.".into())
    } else {
        Ok(name.into())
    }
}
fn valid_collection_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() || name.len() > 200 {
        Err("Nom de playlist invalide.".into())
    } else {
        Ok(name.into())
    }
}
fn ensure_identity_in(tx: &rusqlite::Transaction<'_>, id: &str) -> Result<(), String> {
    let exists: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM cc_media_identity WHERE stable_id=?1)",
            [id],
            |r| r.get(0),
        )
        .map_err(db_error)?;
    exists
        .then_some(())
        .ok_or_else(|| "Vidéo introuvable dans le catalogue.".into())
}
fn state(conn: &Connection, p: &str, m: &str) -> Result<PersonalState, String> {
    conn.query_row("SELECT position_ms,final_watched_ms,watched,rating FROM cc_profile_state WHERE profile_id=?1 AND stable_media_id=?2",params![p,m],|r|Ok(PersonalState{position_ms:r.get::<_,i64>(0)? as u64,final_watched_ms:r.get::<_,i64>(1)? as u64,watched:r.get(2)?,rating:r.get::<_,Option<i64>>(3)?.map(|v|v as u8)})).optional().map_err(db_error).map(|v|v.unwrap_or_default())
}
fn state_in(tx: &rusqlite::Transaction<'_>, p: &str, m: &str) -> Result<PersonalState, String> {
    tx.query_row("SELECT position_ms,final_watched_ms,watched,rating FROM cc_profile_state WHERE profile_id=?1 AND stable_media_id=?2",params![p,m],|r|Ok(PersonalState{position_ms:r.get::<_,i64>(0)? as u64,final_watched_ms:r.get::<_,i64>(1)? as u64,watched:r.get(2)?,rating:r.get::<_,Option<i64>>(3)?.map(|v|v as u8)})).optional().map_err(db_error).map(|v|v.unwrap_or_default())
}
fn upsert_state(
    tx: &rusqlite::Transaction<'_>,
    p: &str,
    m: &str,
    position: Option<u64>,
    final_ms: Option<Option<u64>>,
    watched: Option<bool>,
    rating: Option<Option<u8>>,
) -> Result<(), String> {
    let old = state_in(tx, p, m)?;
    tx.execute("INSERT INTO cc_profile_state(profile_id,stable_media_id,position_ms,final_watched_ms,watched,rating) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(profile_id,stable_media_id) DO UPDATE SET position_ms=excluded.position_ms,final_watched_ms=excluded.final_watched_ms,watched=excluded.watched,rating=excluded.rating",params![p,m,position.unwrap_or(old.position_ms) as i64,final_ms.flatten().unwrap_or(old.final_watched_ms) as i64,watched.unwrap_or(old.watched),rating.flatten().or(old.rating)]).map_err(db_error)?;
    Ok(())
}
fn playlist_members(conn: &Connection, id: &str) -> Result<Vec<String>, String> {
    let mut q=conn.prepare("SELECT stable_media_id FROM cc_playlist_members WHERE playlist_id=?1 ORDER BY position").map_err(db_error)?;
    q.query_map([id], |r| r.get(0))
        .map_err(db_error)?
        .collect::<Result<_, _>>()
        .map_err(db_error)
}
fn playlist_members_in(tx: &rusqlite::Transaction<'_>, id: &str) -> Result<Vec<String>, String> {
    let mut q=tx.prepare("SELECT stable_media_id FROM cc_playlist_members WHERE playlist_id=?1 ORDER BY position").map_err(db_error)?;
    q.query_map([id], |r| r.get(0))
        .map_err(db_error)?
        .collect::<Result<_, _>>()
        .map_err(db_error)
}
fn scrub_media(media: &Media) -> Value {
    let mut value = serde_json::to_value(media).unwrap_or(Value::Null);
    if let Value::Object(ref mut object) = value {
        object.remove("path");
        object.remove("watched");
        object.remove("rating");
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    fn media(id: &str, path: &str) -> Media {
        Media {
            id: id.into(),
            path: path.into(),
            title: id.into(),
            category: "film".into(),
            year: None,
            series: None,
            season: None,
            episode: None,
            genres: vec![],
            overview: String::new(),
            poster: None,
            poster_custom: false,
            runtime_minutes: None,
            cast: vec![],
            directors: vec![],
            series_overview: String::new(),
            series_metadata: None,
            series_manual: false,
            rating: None,
            saga: None,
            watched: false,
            available: true,
            added_at: 0,
            tmdb_id: None,
        }
    }
    fn store() -> Store {
        let d = tempdir().unwrap();
        let path = d.keep().join("catalog.sqlite");
        let mut s = Store::open(&path).unwrap();
        s.snapshot(vec![
            media("Films/A.mkv", "/nas/Films/A.mkv"),
            media("Films/B.mkv", "/nas/Films/B.mkv"),
        ])
        .unwrap();
        s
    }
    #[test]
    fn profiles_are_isolated_and_renaming_keeps_identity() {
        let mut s = store();
        let a = s.create_profile("A").unwrap();
        let b = s.create_profile("B").unwrap();
        let id = s.catalog(&a.id).unwrap()[0].id.clone();
        s.set_rating(&a.id, &id, Some(5), "video").unwrap();
        s.rename_profile(&a.id, "Alice").unwrap();
        assert_eq!(s.media(&a.id, &id).unwrap().rating, Some(5));
        assert_eq!(s.media(&b.id, &id).unwrap().rating, None);
    }
    #[test]
    fn playlists_order_conflicts_without_partial_write() {
        let mut s = store();
        let p = s.create_profile("A").unwrap();
        let ids: Vec<_> = s
            .catalog(&p.id)
            .unwrap()
            .into_iter()
            .map(|m| m.id)
            .collect();
        let list = s.create_collection(&p.id, "Soirée", "playlist").unwrap();
        s.collection_member_add_bulk(&p.id, &list.id, &ids).unwrap();
        s.reorder_playlist(&p.id, &list.id, &ids, &[ids[1].clone(), ids[0].clone()])
            .unwrap();
        assert!(s.reorder_playlist(&p.id, &list.id, &ids, &ids).is_err());
        let got = s
            .collections(&p.id)
            .unwrap()
            .into_iter()
            .find(|c| c.id == list.id)
            .unwrap();
        assert_eq!(got.media_ids, vec![ids[1].clone(), ids[0].clone()]);
    }
    #[test]
    fn watch_threshold_and_manual_reset_preserve_resume() {
        let mut s = store();
        let p = s.create_profile("A").unwrap();
        let id = s.catalog(&p.id).unwrap()[0].id.clone();
        assert!(
            !s.save_progress(&p.id, &id, 50, 9999, 100000)
                .unwrap()
                .watched
        );
        assert!(s.save_progress(&p.id, &id, 60, 1, 100000).unwrap().watched);
        s.set_watched(&p.id, &id, false).unwrap();
        let state = state(&s.personal, &p.id, &id).unwrap();
        assert!(!state.watched);
        assert_eq!(state.final_watched_ms, 0);
        assert_eq!(state.position_ms, 60);
    }
    #[test]
    fn deleting_profile_is_personal_only() {
        let mut s = store();
        let p = s.create_profile("A").unwrap();
        let id = s.catalog(&p.id).unwrap()[0].id.clone();
        s.delete_profile(&p.id).unwrap();
        assert!(s.media_path(&id).unwrap().is_absolute());
        assert_eq!(s.catalogue.list().unwrap().len(), 2);
    }
    #[test]
    fn export_is_portable_and_has_no_absolute_path_or_lease() {
        let mut s = store();
        let p = s.create_profile("A").unwrap();
        let id = s.catalog(&p.id).unwrap()[0].id.clone();
        s.set_rating(&p.id, &id, Some(3), "video").unwrap();
        let doc = s.export_json().unwrap();
        assert!(doc.to_string().contains("format_version"));
        assert!(!doc.to_string().contains("/nas/"));
        assert_eq!(s.preview_import(&doc).unwrap().1, 1);
    }
    #[test]
    fn prepared_import_round_trips_personal_state_and_is_idempotent() {
        let mut source = store();
        let source_profile = source.create_profile("Source").unwrap();
        let source_id = source.catalog(&source_profile.id).unwrap()[0].id.clone();
        source
            .set_rating(&source_profile.id, &source_id, Some(4), "video")
            .unwrap();
        let document = source.export_json().unwrap();

        let mut destination = store();
        let destination_profile = destination.create_profile("Destination").unwrap();
        let mut mapping = HashMap::new();
        mapping.insert(source_profile.id, destination_profile.id.clone());
        assert!(
            destination
                .import_prepared_json(&document, &mapping)
                .unwrap()
        );
        let destination_id = destination.catalog(&destination_profile.id).unwrap()[0]
            .id
            .clone();
        assert_eq!(
            destination
                .media(&destination_profile.id, &destination_id)
                .unwrap()
                .rating,
            Some(4)
        );
        assert!(
            !destination
                .import_prepared_json(&document, &mapping)
                .unwrap()
        );
    }
}
