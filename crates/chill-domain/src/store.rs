//! Profile-scoped persistence over the legacy common catalogue.
//!
//! The catalogue continues to own provider metadata and physical locations.  This
//! module deliberately never opens or walks a media path: `media_path` is the
//! narrow server-side escape hatch and all catalogue views expose relative paths.

use crate::{
    NextVideo, PersonalState, PlaybackContext, Preferences, Profile, Progress, valid_speed,
};
use chill_catalog::{
    database::Database,
    model::{Collection, Media, MediaPatch},
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};
use uuid::Uuid;

const EXPORT_FORMAT_VERSION: u64 = 1;
const WATCHED_THRESHOLD_MS: u64 = 10_000;
const MAX_IN_MEMORY_EXPORT_BYTES: usize = 1024 * 1024;
pub const MAX_CATALOG_PAGE: usize = 100;
pub const MAX_EXPORT_POSTER_BYTES: u64 = 8 * 1024 * 1024;
#[derive(Debug, Clone)]
pub struct ArchivePoster {
    pub reference: String,
    pub relative: PathBuf,
}
#[derive(Debug, Clone)]
pub struct ArchiveSnapshot {
    pub document: Value,
    pub posters: Vec<ArchivePoster>,
}

#[derive(Debug, Clone, Default)]
pub struct CatalogQuery {
    pub offset: usize,
    pub limit: usize,
    pub category: Option<String>,
    pub title_contains: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CatalogPage {
    pub items: Vec<Media>,
    pub has_more: bool,
}

/// Personal playlist view.  The revision is deliberately separate from the
/// common legacy `Collection`: clients must supply it for every mutation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PlaylistView {
    pub id: String,
    pub name: String,
    pub revision: u64,
    pub media_ids: Vec<String>,
}
#[derive(Debug, Clone, serde::Serialize)]
pub struct SagaView {
    pub id: String,
    pub name: String,
    pub revision: u64,
    pub media_ids: Vec<String>,
    pub poster: Option<String>,
}

pub struct Store {
    catalogue: Database,
    pub(crate) personal: Connection,
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
             CREATE TABLE IF NOT EXISTS cc_media_revision (
                stable_id TEXT PRIMARY KEY REFERENCES cc_media_identity(stable_id) ON DELETE CASCADE,
                revision INTEGER NOT NULL DEFAULT 1
             );
             CREATE TABLE IF NOT EXISTS cc_profile_state (
                profile_id TEXT NOT NULL REFERENCES cc_profiles(id) ON DELETE CASCADE,
                stable_media_id TEXT NOT NULL REFERENCES cc_media_identity(stable_id),
                position_ms INTEGER NOT NULL DEFAULT 0 CHECK(position_ms >= 0),
                final_watched_ms INTEGER NOT NULL DEFAULT 0 CHECK(final_watched_ms >= 0),
                watched INTEGER NOT NULL DEFAULT 0 CHECK(watched IN (0,1)),
                rating INTEGER CHECK(rating BETWEEN 1 AND 5),
                last_session_id TEXT,
                last_generation INTEGER,
                last_sequence INTEGER,
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
             CREATE TABLE IF NOT EXISTS cc_catalog_epoch (id INTEGER PRIMARY KEY CHECK(id=1), revision INTEGER NOT NULL);
             INSERT OR IGNORE INTO cc_catalog_epoch(id,revision) VALUES(1,0);
             CREATE TRIGGER IF NOT EXISTS cc_epoch_media_i AFTER INSERT ON media BEGIN UPDATE cc_catalog_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_epoch_media_u AFTER UPDATE ON media BEGIN UPDATE cc_catalog_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_epoch_media_d AFTER DELETE ON media BEGIN UPDATE cc_catalog_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_epoch_profiles_i AFTER INSERT ON cc_profiles BEGIN UPDATE cc_catalog_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_epoch_profiles_u AFTER UPDATE ON cc_profiles BEGIN UPDATE cc_catalog_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_epoch_profiles_d AFTER DELETE ON cc_profiles BEGIN UPDATE cc_catalog_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_epoch_state_i AFTER INSERT ON cc_profile_state BEGIN UPDATE cc_catalog_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_epoch_state_u AFTER UPDATE ON cc_profile_state BEGIN UPDATE cc_catalog_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_epoch_state_d AFTER DELETE ON cc_profile_state BEGIN UPDATE cc_catalog_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_epoch_list_i AFTER INSERT ON cc_playlists BEGIN UPDATE cc_catalog_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_epoch_list_u AFTER UPDATE ON cc_playlists BEGIN UPDATE cc_catalog_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_epoch_list_d AFTER DELETE ON cc_playlists BEGIN UPDATE cc_catalog_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_epoch_members_i AFTER INSERT ON cc_playlist_members BEGIN UPDATE cc_catalog_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_epoch_members_u AFTER UPDATE ON cc_playlist_members BEGIN UPDATE cc_catalog_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_epoch_members_d AFTER DELETE ON cc_playlist_members BEGIN UPDATE cc_catalog_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_epoch_collections_i AFTER INSERT ON collections BEGIN UPDATE cc_catalog_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_epoch_collections_u AFTER UPDATE ON collections BEGIN UPDATE cc_catalog_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_epoch_collections_d AFTER DELETE ON collections BEGIN UPDATE cc_catalog_epoch SET revision=revision+1 WHERE id=1; END;
             PRAGMA user_version=3;"
        ).map_err(db_error)?;
        // Existing development databases may predate ordered progress guards.
        add_column_if_missing(&personal, "cc_profile_state", "last_session_id TEXT")?;
        add_column_if_missing(&personal, "cc_profile_state", "last_generation INTEGER")?;
        add_column_if_missing(&personal, "cc_profile_state", "last_sequence INTEGER")?;
        // The epoch guards the entire imported state, including writes from a
        // second connection (scanner). Every trigger is transactional.
        for table in [
            "cc_media_identity",
            "cc_media_revision",
            "cc_preferences",
            "cc_series_speed",
            "collection_members",
            "collection_positions",
            "collection_artwork",
            "collection_revisions",
        ] {
            for (suffix, event) in [("i", "INSERT"), ("u", "UPDATE"), ("d", "DELETE")] {
                personal.execute_batch(&format!("CREATE TRIGGER IF NOT EXISTS cc_epoch_{table}_{suffix} AFTER {event} ON {table} BEGIN UPDATE cc_catalog_epoch SET revision=revision+1 WHERE id=1; END;")).map_err(db_error)?;
            }
        }
        personal.execute_batch("CREATE TABLE IF NOT EXISTS cc_import_receipts(export_id TEXT NOT NULL,mapping TEXT NOT NULL,profiles TEXT NOT NULL DEFAULT '{}',PRIMARY KEY(export_id,mapping));").map_err(db_error)?;
        add_column_if_missing(
            &personal,
            "cc_import_receipts",
            "profiles TEXT NOT NULL DEFAULT '{}'",
        )?;
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

    /// Cheap stale-plan token maintained by SQLite triggers across Store connections.
    pub fn catalog_revision(&self) -> Result<u64, String> {
        self.personal
            .query_row(
                "SELECT revision FROM cc_catalog_epoch WHERE id=1",
                [],
                |r| r.get::<_, i64>(0),
            )
            .map(|value| value as u64)
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
        let revision = self.profile(id)?.ok_or("PROFILE_NOT_FOUND")?.revision;
        self.rename_profile_at_revision(id, name, revision)
    }

    pub fn rename_profile_at_revision(
        &self,
        id: &str,
        name: &str,
        revision: u64,
    ) -> Result<Profile, String> {
        let revision = i64::try_from(revision).map_err(|_| "PROFILE_CONFLICT")?;
        let name = valid_profile_name(name)?;
        if self
            .personal
            .execute(
                "UPDATE cc_profiles SET name=?1,revision=revision+1 WHERE id=?2 AND revision=?3",
                params![name, id, revision],
            )
            .map_err(profile_collision)?
            != 1
        {
            return Err(if self.profile(id)?.is_some() {
                "PROFILE_CONFLICT"
            } else {
                "PROFILE_NOT_FOUND"
            }
            .into());
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

    /// Bounded server query seam. The HTTP layer must use this rather than
    /// `catalog`, which remains a compatibility/admin helper.
    pub fn catalog_page(
        &mut self,
        profile_id: &str,
        query: &CatalogQuery,
    ) -> Result<CatalogPage, String> {
        self.require_profile(profile_id)?;
        let limit = if query.limit == 0 {
            MAX_CATALOG_PAGE
        } else {
            query.limit
        };
        if limit > MAX_CATALOG_PAGE || query.title_contains.as_ref().is_some_and(|v| v.len() > 500)
        {
            return Err("Filtre ou page catalogue invalide.".into());
        }
        let category = query.category.as_deref();
        if category.is_some_and(|v| !chill_catalog::model::CATEGORIES.contains(&v)) {
            return Err("Catégorie inconnue.".into());
        }
        let title = query.title_contains.as_deref().unwrap_or("");
        let mut statement = self
            .personal
            .prepare(
                "SELECT data FROM media
             WHERE (?1 IS NULL OR json_extract(data,'$.category')=?1)
             AND (?2='' OR json_extract(data,'$.title') LIKE '%' || ?2 || '%')
             ORDER BY id LIMIT ?3 OFFSET ?4",
            )
            .map_err(db_error)?;
        let rows = statement
            .query_map(
                params![category, title, (limit + 1) as i64, query.offset as i64],
                |r| r.get::<_, String>(0),
            )
            .map_err(db_error)?;
        let found = rows
            .map(|row| {
                serde_json::from_str::<Media>(&row?).map_err(|_| rusqlite::Error::InvalidQuery)
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(db_error)?;
        self.ensure_identities(&found)?;
        let has_more = found.len() > limit;
        let items = found
            .into_iter()
            .take(limit)
            .map(|m| self.profile_view(profile_id, m))
            .collect::<Result<_, _>>()?;
        Ok(CatalogPage { items, has_more })
    }

    pub fn media(&mut self, profile_id: &str, stable_id: &str) -> Result<Media, String> {
        self.require_profile(profile_id)?;
        let legacy = self.legacy_id(stable_id)?;
        self.profile_view(profile_id, self.catalogue.get(&legacy)?)
    }

    pub fn series(&mut self, profile_id: &str, name: &str) -> Result<Vec<Media>, String> {
        self.require_profile(profile_id)?;
        if name.is_empty() || name.len() > 500 {
            return Err("Série invalide.".into());
        }
        let mut members: Vec<_> = self
            .catalog(profile_id)?
            .into_iter()
            .filter(|m| m.category == "serie" && m.series.as_deref().unwrap_or(&m.title) == name)
            .collect();
        members.sort_by_key(|m| {
            (
                m.season.unwrap_or(i32::MAX),
                m.episode.unwrap_or(i32::MAX),
                m.title.clone(),
            )
        });
        if members.is_empty() {
            return Err("Série introuvable dans le catalogue.".into());
        }
        Ok(members)
    }

    /// Absolute paths stay within the server process and are never serialized in catalogue views.
    pub fn media_path(&self, stable_id: &str) -> Result<PathBuf, String> {
        let legacy = self.legacy_id(stable_id)?;
        Ok(PathBuf::from(self.catalogue.get(&legacy)?.path))
    }

    pub fn personal_state(
        &self,
        profile_id: &str,
        stable_id: &str,
    ) -> Result<PersonalState, String> {
        self.require_profile(profile_id)?;
        self.legacy_id(stable_id)?;
        state(&self.personal, profile_id, stable_id)
    }

    /// Context is server-derived: a client cannot attach an arbitrary playlist,
    /// saga or series to influence autoplay.
    pub fn validate_playback_context(
        &mut self,
        profile: &str,
        media: &str,
        context: &PlaybackContext,
    ) -> Result<(), String> {
        let item = self.media(profile, media)?;
        match (context.kind.as_str(), context.id.as_deref()) {
            ("standalone", None) => Ok(()),
            ("series", Some(name))
                if item.category == "serie"
                    && item.series.as_deref().unwrap_or(&item.title) == name =>
            {
                Ok(())
            }
            ("playlist", Some(id)) => {
                let list = self.playlist(profile, id)?;
                if list.media_ids.iter().any(|v| v == media) {
                    Ok(())
                } else {
                    Err("Cette vidéo n’appartient pas à cette playlist.".into())
                }
            }
            ("saga", Some(id)) => {
                let saga = self
                    .collections(profile)?
                    .into_iter()
                    .find(|c| c.kind == "saga" && c.id == id)
                    .ok_or("Saga introuvable.")?;
                if saga.media_ids.iter().any(|v| v == media) {
                    Ok(())
                } else {
                    Err("Cette vidéo n’appartient pas à cette saga.".into())
                }
            }
            _ => Err("Contexte de lecture invalide.".into()),
        }
    }

    pub fn next_video(
        &mut self,
        profile: &str,
        media: &str,
        context: &PlaybackContext,
    ) -> Result<NextVideo, String> {
        self.validate_playback_context(profile, media, context)?;
        let current = self.media(profile, media)?;
        let next = match (context.kind.as_str(), context.id.as_deref()) {
            ("playlist", Some(id)) => {
                let p = self.playlist(profile, id)?;
                p.media_ids
                    .iter()
                    .position(|v| v == media)
                    .and_then(|n| p.media_ids.get(n + 1))
                    .cloned()
            }
            ("saga", Some(id)) => self
                .collections(profile)?
                .into_iter()
                .find(|c| c.kind == "saga" && c.id == id)
                .and_then(|c| {
                    c.media_ids
                        .iter()
                        .position(|v| v == media)
                        .and_then(|n| c.media_ids.get(n + 1))
                        .cloned()
                }),
            ("series", _) => {
                if current.category != "serie"
                    || current.season.is_none()
                    || current.episode.is_none()
                {
                    None
                } else {
                    self.series(profile, current.series.as_deref().unwrap_or(&current.title))?
                        .into_iter()
                        .find(|m| (m.season, m.episode) > (current.season, current.episode))
                        .map(|m| m.id)
                }
            }
            _ => None,
        };
        let item = match next {
            Some(id) => Some(self.media(profile, &id)?),
            None => None,
        };
        let new_season = context.kind == "series"
            && current.season.is_some()
            && item.as_ref().is_some_and(|m| m.season != current.season);
        let available = item.as_ref().is_some_and(|m| m.available);
        Ok(NextVideo {
            media: item,
            new_season,
            available,
        })
    }

    pub fn snapshot(&mut self, discovered: Vec<Media>) -> Result<i64, String> {
        self.catalogue.snapshot_with(discovered, insert_identities)
    }

    /// SQLite checks before commit, including autocommit statements, prevent a
    /// cancelled/deadline-expired interactive job from committing late writes.
    pub fn cancellation_probe(
        &self,
        probe: Option<std::sync::Arc<dyn Fn() -> bool + Send + Sync>>,
    ) -> Result<(), String> {
        self.catalogue.cancellation_probe(probe.clone())?;
        match probe {
            Some(probe) => {
                let commit = probe.clone();
                self.personal
                    .commit_hook(Some(move || commit()))
                    .map_err(db_error)?;
                self.personal
                    .progress_handler(1000, Some(move || probe()))
                    .map_err(db_error)?;
            }
            None => {
                self.personal
                    .commit_hook(None::<fn() -> bool>)
                    .map_err(db_error)?;
                self.personal
                    .progress_handler(0, None::<fn() -> bool>)
                    .map_err(db_error)?;
            }
        }
        Ok(())
    }

    /// Explicit administrator-confirmed reassociation. This never guesses from
    /// title/metadata and does not touch NAS files; it moves the stable identity
    /// to a new already-scanned relative location.
    pub fn confirm_media_reassociation(
        &self,
        stable_id: &str,
        new_relative_path: &str,
    ) -> Result<(), String> {
        if new_relative_path.is_empty()
            || Path::new(new_relative_path).is_absolute()
            || new_relative_path.split('/').any(|part| part == "..")
        {
            return Err("Nouvelle localisation relative invalide.".into());
        }
        // Prove the target was discovered; no title-based fallback is allowed.
        self.catalogue.get(new_relative_path)?;
        let tx = self.personal.unchecked_transaction().map_err(db_error)?;
        let target_identity: Option<String> = tx
            .query_row(
                "SELECT stable_id FROM cc_media_identity WHERE legacy_id=?1 AND stable_id<>?2",
                params![new_relative_path, stable_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(db_error)?;
        if let Some(target_identity) = target_identity {
            let referenced: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM cc_profile_state WHERE stable_media_id=?1) OR EXISTS(SELECT 1 FROM cc_playlist_members WHERE stable_media_id=?1)",
                [&target_identity], |r| r.get(0)
            ).map_err(db_error)?;
            if referenced {
                return Err("Cette localisation possède déjà une identité stable utilisée.".into());
            }
            tx.execute(
                "DELETE FROM cc_media_identity WHERE stable_id=?1",
                [&target_identity],
            )
            .map_err(db_error)?;
        }
        if tx
            .execute(
                "UPDATE cc_media_identity SET legacy_id=?1,relative_path=?1 WHERE stable_id=?2",
                params![new_relative_path, stable_id],
            )
            .map_err(db_error)?
            != 1
        {
            return Err("Vidéo introuvable dans le catalogue.".into());
        }
        tx.commit().map_err(db_error)
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

    pub fn media_revision(&self, stable_id: &str) -> Result<u64, String> {
        self.legacy_id(stable_id)?;
        self.personal
            .query_row(
                "SELECT revision FROM cc_media_revision WHERE stable_id=?1",
                [stable_id],
                |r| r.get::<_, i64>(0),
            )
            .optional()
            .map_err(db_error)?
            .map(|v| v as u64)
            .ok_or_else(|| "Vidéo introuvable dans le catalogue.".into())
    }

    pub fn update_media_at_revision(
        &mut self,
        profile: &str,
        stable_id: &str,
        expected: u64,
        patch: &MediaPatch,
    ) -> Result<(Media, u64), String> {
        self.require_profile(profile)?;
        if patch.rating.is_some() || patch.watched.is_some() {
            return Err("La note et le vu sont propres au profil.".into());
        }
        let legacy = self.legacy_id(stable_id)?;
        let tx = self.personal.unchecked_transaction().map_err(db_error)?;
        let revision: i64 = tx
            .query_row(
                "SELECT revision FROM cc_media_revision WHERE stable_id=?1",
                [stable_id],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        if revision as u64 != expected {
            return Err("La fiche a changé entre-temps.".into());
        }
        let data: String = tx
            .query_row("SELECT data FROM media WHERE id=?1", [&legacy], |r| {
                r.get(0)
            })
            .map_err(db_error)?;
        let mut updated: Media = serde_json::from_str(&data)
            .map_err(|_| "Catalogue local endommagé : aucune donnée remplacée.".to_string())?;
        patch.apply(&mut updated)?;
        tx.execute(
            "UPDATE media SET data=?1 WHERE id=?2",
            params![
                serde_json::to_string(&updated).map_err(|_| "Métadonnées invalides.")?,
                legacy
            ],
        )
        .map_err(db_error)?;
        tx.execute(
            "UPDATE cc_media_revision SET revision=revision+1 WHERE stable_id=?1",
            [stable_id],
        )
        .map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        Ok((self.profile_view(profile, updated)?, revision as u64 + 1))
    }
    pub fn set_custom_poster(
        &mut self,
        profile: &str,
        stable: &str,
        expected: u64,
        bytes: &[u8],
        cache: &Path,
    ) -> Result<(Media, u64), String> {
        self.require_profile(profile)?;
        if bytes.len() > 8 * 1024 * 1024 {
            return Err("Image trop volumineuse.".into());
        };
        let legacy = self.legacy_id(stable)?;
        let tx = self.personal.unchecked_transaction().map_err(db_error)?;
        let revision: i64 = tx
            .query_row(
                "SELECT revision FROM cc_media_revision WHERE stable_id=?1",
                [stable],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        if revision as u64 != expected {
            return Err("La fiche a changé entre-temps.".into());
        };
        // Store validates JPEG/PNG/WebP before any database pointer is changed.
        let poster = chill_catalog::poster::store(bytes, cache)?;
        let data: String = tx
            .query_row("SELECT data FROM media WHERE id=?1", [&legacy], |r| {
                r.get(0)
            })
            .map_err(db_error)?;
        let mut item: Media = serde_json::from_str(&data)
            .map_err(|_| "Catalogue local endommagé : aucune donnée remplacée.".to_string())?;
        item.poster = Some(poster);
        item.poster_custom = true;
        tx.execute(
            "UPDATE media SET data=?1 WHERE id=?2",
            params![
                serde_json::to_string(&item).map_err(|_| "Métadonnées invalides.")?,
                legacy
            ],
        )
        .map_err(db_error)?;
        tx.execute(
            "UPDATE cc_media_revision SET revision=revision+1 WHERE stable_id=?1",
            [stable],
        )
        .map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        Ok((self.profile_view(profile, item)?, revision as u64 + 1))
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
        let _ = (
            profile_id,
            stable_id,
            position_ms,
            final_elapsed_ms,
            duration_ms,
        );
        Err("La progression doit être validée par le bail de lecture actif.".into())
    }

    /// Persists a progress event only after the server actor has verified the
    /// active lease, media revision and timing against its monotonic clock.
    /// This store additionally rejects replayed/out-of-order events.
    pub fn save_validated_progress(
        &self,
        profile_id: &str,
        stable_id: &str,
        progress: &Progress,
    ) -> Result<PersonalState, String> {
        self.require_profile(profile_id)?;
        if progress.media_id != stable_id
            || progress.duration_ms == 0
            || progress.position_ms > progress.duration_ms
            || !valid_speed(progress.speed)
        {
            return Err("Progression validée invalide.".into());
        }
        // A resume position is meaningful for every ordered, lease-validated event,
        // including pause, seek and early playback. Only uninterrupted playback in
        // the final tenth may add watched credit.
        let credit_eligible = progress.playing
            && !progress.buffering
            && !progress.seek
            && progress.position_ms >= crate::final_tenth_start(progress.duration_ms);
        // Real time in the final tenth can be twice the media time at ×0.5.
        // The server separately bounds each credit by its monotonic clock.
        let maximum_final_time = (progress.duration_ms
            - crate::final_tenth_start(progress.duration_ms))
        .saturating_mul(2);
        if progress.final_elapsed_ms > maximum_final_time
            || (!credit_eligible && progress.final_elapsed_ms != 0)
        {
            return Err("Crédit de visionnage final invalide.".into());
        }
        self.legacy_id(stable_id)?;
        let tx = self.personal.unchecked_transaction().map_err(db_error)?;
        let old = state_in(&tx, profile_id, stable_id)?;
        let previous: Option<(String, u64, u64)> = tx.query_row(
            "SELECT last_session_id,last_generation,last_sequence FROM cc_profile_state WHERE profile_id=?1 AND stable_media_id=?2 AND last_session_id IS NOT NULL",
            params![profile_id,stable_id], |r| Ok((r.get(0)?, r.get::<_,i64>(1)? as u64, r.get::<_,i64>(2)? as u64))
        ).optional().map_err(db_error)?;
        if previous.is_some_and(|(session, generation, sequence)| {
            session == progress.session_id
                && (generation != progress.generation || progress.sequence <= sequence)
        }) {
            return Err("Événement de lecture périmé ou rejoué.".into());
        }
        let final_watched_ms = old.final_watched_ms.saturating_add(if credit_eligible {
            progress.final_elapsed_ms
        } else {
            0
        });
        let watched = old.watched || final_watched_ms >= WATCHED_THRESHOLD_MS;
        upsert_state(
            &tx,
            profile_id,
            stable_id,
            Some(progress.position_ms),
            Some(Some(final_watched_ms)),
            Some(watched),
            None,
        )?;
        tx.execute("UPDATE cc_profile_state SET last_session_id=?1,last_generation=?2,last_sequence=?3 WHERE profile_id=?4 AND stable_media_id=?5",params![progress.session_id,progress.generation as i64,progress.sequence as i64,profile_id,stable_id]).map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        Ok(PersonalState {
            position_ms: progress.position_ms,
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

    pub fn playlists(&self, profile_id: &str) -> Result<Vec<PlaylistView>, String> {
        self.require_profile(profile_id)?;
        let mut stmt = self
            .personal
            .prepare(
                "SELECT id,name,revision FROM cc_playlists WHERE profile_id=?1 ORDER BY name,id",
            )
            .map_err(db_error)?;
        stmt.query_map([profile_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })
        .map_err(db_error)?
        .map(|row| {
            let (id, name, revision) = row.map_err(db_error)?;
            Ok(PlaylistView {
                media_ids: self.playlist_members(&id)?,
                id,
                name,
                revision: revision as u64,
            })
        })
        .collect()
    }
    pub fn sagas(&mut self) -> Result<Vec<SagaView>, String> {
        self.catalogue
            .collections()?
            .into_iter()
            .filter(|c| c.kind == "saga")
            .map(|c| {
                let media_ids = c
                    .media_ids
                    .iter()
                    .map(|legacy| self.stable_id(legacy))
                    .collect::<Result<Vec<_>, _>>()?;
                let poster = c
                    .poster
                    .filter(|p| !Path::new(p).is_absolute() && !p.starts_with("file:"));
                Ok(SagaView {
                    revision: self.catalogue.saga_revision(&c.id)?,
                    id: c.id,
                    name: c.name,
                    media_ids,
                    poster,
                })
            })
            .collect()
    }
    pub fn saga(&mut self, id: &str) -> Result<SagaView, String> {
        self.sagas()?
            .into_iter()
            .find(|s| s.id == id)
            .ok_or_else(|| "Saga introuvable.".into())
    }
    pub fn create_saga(&mut self, name: &str) -> Result<SagaView, String> {
        let id = self
            .catalogue
            .create_collection(name, "saga")?
            .into_iter()
            .find(|c| c.kind == "saga" && c.name == name.trim())
            .ok_or("Saga introuvable.")?
            .id;
        self.saga(&id)
    }
    pub fn rename_saga_at_revision(
        &mut self,
        id: &str,
        name: &str,
        revision: u64,
    ) -> Result<SagaView, String> {
        self.catalogue.rename_saga_at_revision(id, name, revision)?;
        self.saga(id)
    }
    pub fn saga_members_at_revision(
        &mut self,
        id: &str,
        media: &[String],
        revision: u64,
        add: bool,
    ) -> Result<SagaView, String> {
        let legacy = media
            .iter()
            .map(|stable| self.legacy_id(stable))
            .collect::<Result<Vec<_>, _>>()?;
        self.catalogue
            .saga_members_at_revision(id, &legacy, revision, add)?;
        self.saga(id)
    }
    pub fn reorder_saga_at_revision(
        &mut self,
        id: &str,
        revision: u64,
        expected: &[String],
        desired: &[String],
    ) -> Result<SagaView, String> {
        let expected = expected
            .iter()
            .map(|stable| self.legacy_id(stable))
            .collect::<Result<Vec<_>, _>>()?;
        let desired = desired
            .iter()
            .map(|stable| self.legacy_id(stable))
            .collect::<Result<Vec<_>, _>>()?;
        self.catalogue
            .reorder_saga_at_revision(id, revision, &expected, &desired)?;
        self.saga(id)
    }
    pub fn delete_saga_at_revision(&mut self, id: &str, revision: u64) -> Result<(), String> {
        self.catalogue.delete_saga_at_revision(id, revision)
    }

    pub fn playlist(&self, profile_id: &str, id: &str) -> Result<PlaylistView, String> {
        self.require_playlist(profile_id, id)?;
        let (name, revision): (String, i64) = self
            .personal
            .query_row(
                "SELECT name,revision FROM cc_playlists WHERE id=?1 AND profile_id=?2",
                params![id, profile_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(db_error)?;
        Ok(PlaylistView {
            id: id.into(),
            name,
            revision: revision as u64,
            media_ids: self.playlist_members(id)?,
        })
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

    pub fn create_playlist_with_members(
        &self,
        profile_id: &str,
        name: &str,
        media_ids: &[String],
    ) -> Result<PlaylistView, String> {
        self.require_profile(profile_id)?;
        if media_ids.len() > 1000 {
            return Err("Sélectionnez au plus 1 000 vidéos.".into());
        }
        let name = valid_collection_name(name)?;
        let id = Uuid::new_v4().to_string();
        let tx = self.personal.unchecked_transaction().map_err(db_error)?;
        tx.execute(
            "INSERT INTO cc_playlists(id,profile_id,name) VALUES(?1,?2,?3)",
            params![id, profile_id, name],
        )
        .map_err(|_| "Une playlist porte déjà ce nom pour ce profil.".to_string())?;
        let mut seen = HashSet::new();
        for (position, media) in media_ids.iter().enumerate() {
            if !seen.insert(media) {
                return Err("Sélection invalide : vidéo dupliquée.".into());
            }
            ensure_identity_in(&tx, media)?;
            tx.execute("INSERT INTO cc_playlist_members(playlist_id,stable_media_id,position) VALUES(?1,?2,?3)",params![id,media,position as i64]).map_err(db_error)?;
        }
        tx.commit().map_err(db_error)?;
        self.playlist(profile_id, &id)
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

    pub fn collection_member_add_bulk_at_revision(
        &self,
        profile_id: &str,
        collection_id: &str,
        media_ids: &[String],
        expected: u64,
    ) -> Result<(), String> {
        let current = self.playlist(profile_id, collection_id)?;
        if current.revision != expected {
            return Err("La playlist a changé entre-temps.".into());
        }
        self.collection_member_add_bulk(profile_id, collection_id, media_ids)
    }

    pub fn collection_member_remove(
        &self,
        profile_id: &str,
        collection_id: &str,
        stable_id: &str,
    ) -> Result<(), String> {
        self.collection_member_remove_at_revision(profile_id, collection_id, stable_id, None)
    }

    pub fn collection_member_remove_at_revision(
        &self,
        profile_id: &str,
        collection_id: &str,
        stable_id: &str,
        expected_revision: Option<u64>,
    ) -> Result<(), String> {
        self.require_playlist(profile_id, collection_id)?;
        self.legacy_id(stable_id)?;
        let tx = self.personal.unchecked_transaction().map_err(db_error)?;
        let revision: u64 = tx
            .query_row(
                "SELECT revision FROM cc_playlists WHERE id=?1",
                [collection_id],
                |r| r.get::<_, i64>(0),
            )
            .map_err(db_error)? as u64;
        if expected_revision.is_some_and(|value| value != revision) {
            return Err("La playlist a changé entre-temps.".into());
        }
        tx.execute(
            "DELETE FROM cc_playlist_members WHERE playlist_id=?1 AND stable_media_id=?2",
            params![collection_id, stable_id],
        )
        .map_err(db_error)?;
        let members = playlist_members_in(&tx, collection_id)?;
        for (position, member) in members.iter().enumerate() {
            tx.execute("UPDATE cc_playlist_members SET position=?1 WHERE playlist_id=?2 AND stable_media_id=?3",params![position as i64,collection_id,member]).map_err(db_error)?;
        }
        tx.execute(
            "UPDATE cc_playlists SET revision=revision+1 WHERE id=?1",
            [collection_id],
        )
        .map_err(db_error)?;
        tx.commit().map_err(db_error)
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
    pub fn rename_playlist_at_revision(
        &self,
        profile: &str,
        id: &str,
        name: &str,
        expected: u64,
    ) -> Result<(), String> {
        self.require_playlist(profile, id)?;
        let name = valid_collection_name(name)?;
        let tx = self.personal.unchecked_transaction().map_err(db_error)?;
        let duplicate:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM cc_playlists WHERE profile_id=?1 AND name=?2 AND id<>?3)",params![profile,name,id],|r|r.get(0)).map_err(db_error)?;
        if duplicate {
            return Err("PLAYLIST_NAME_CONFLICT".into());
        }
        if tx.execute("UPDATE cc_playlists SET name=?1,revision=revision+1 WHERE id=?2 AND profile_id=?3 AND revision=?4",params![name,id,profile,expected as i64]).map_err(db_error)? != 1 { return Err("La playlist a changé entre-temps.".into()); }
        tx.commit().map_err(db_error)?;
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
        self.export_json_with_limit(Some(MAX_IN_MEMORY_EXPORT_BYTES))
    }

    /// Archive staging owns its size limits on disk. The legacy API remains
    /// memory-bounded for compatibility.
    pub fn export_archive_snapshot(&mut self) -> Result<ArchiveSnapshot, String> {
        let mut document = self.export_json_with_limit(None)?;
        let media = self.catalogue.list()?;
        let mut posters = Vec::new();
        let data = document
            .get_mut("media")
            .and_then(Value::as_array_mut)
            .ok_or("Export invalide.")?;
        for (row, source) in data.iter_mut().zip(media.iter()) {
            let Some(path) = source
                .poster
                .as_ref()
                .filter(|p| Path::new(p).is_absolute() && !p.starts_with("file:"))
            else {
                continue;
            };
            let meta =
                std::fs::symlink_metadata(path).map_err(|_| "Affiche d’export inaccessible.")?;
            if meta.file_type().is_symlink()
                || !meta.is_file()
                || meta.len() > MAX_EXPORT_POSTER_BYTES
            {
                return Err("Affiche d’export invalide ou trop volumineuse.".into());
            }
            let stable = self.stable_id(&source.id)?;
            let reference = format!("posters/{stable}.bin");
            if let Some(object) = row.get_mut("data").and_then(Value::as_object_mut) {
                object.insert("poster".into(), Value::String(reference.clone()));
            }
            posters.push(ArchivePoster {
                reference,
                relative: PathBuf::from(
                    Path::new(path)
                        .file_name()
                        .ok_or("Affiche d’export invalide.")?,
                ),
            });
        }
        Ok(ArchiveSnapshot { document, posters })
    }

    /// Writes the portable document incrementally.  It deliberately never
    /// builds a JSON `Value` containing the full library.
    pub fn write_archive_json<W: std::io::Write>(
        &mut self,
        out: &mut W,
        export_id: &str,
        poster_root: &Path,
    ) -> Result<Vec<ArchivePoster>, String> {
        self.ensure_identities(&self.catalogue.list()?)?;
        let tx =
            rusqlite::Transaction::new_unchecked(&self.personal, TransactionBehavior::Immediate)
                .map_err(db_error)?;
        let media = self.catalogue.list()?;
        let profiles = self.profiles()?;
        let mut posters = Vec::new();
        out.write_all(br#"{"format_version":1,"export_id":"#)
            .map_err(|_| "Export de staging inaccessible.")?;
        serde_json::to_writer(&mut *out, export_id).map_err(|_| "Export invalide.")?;
        out.write_all(b",\"profiles\":")
            .map_err(|_| "Export de staging inaccessible.")?;
        write_json_array(out, profiles.iter(), |out, profile| {
            serde_json::to_writer(out, profile).map_err(|_| "Export invalide.".into())
        })?;
        out.write_all(b",\"media\":[")
            .map_err(|_| "Export de staging inaccessible.")?;
        for (index, item) in media.iter().enumerate() {
            if index != 0 {
                out.write_all(b",")
                    .map_err(|_| "Export de staging inaccessible.")?;
            }
            let stable = self.stable_id(&item.id)?;
            let mut data = scrub_media(item);
            if let Some(path) = item
                .poster
                .as_ref()
                .and_then(|p| Path::new(p).strip_prefix(poster_root).ok())
            {
                if path
                    .components()
                    .all(|c| matches!(c, std::path::Component::Normal(_)))
                {
                    let reference = format!("posters/{stable}.bin");
                    if let Some(object) = data.as_object_mut() {
                        object.insert("poster".into(), Value::String(reference.clone()));
                    }
                    posters.push(ArchivePoster {
                        reference,
                        relative: path.to_path_buf(),
                    });
                }
            } else if let Some(object) = data.as_object_mut() {
                object.remove("poster");
            }
            serde_json::to_writer(
                &mut *out,
                &json!({"id":stable,"relative_path":item.id,"data":data}),
            )
            .map_err(|_| "Export invalide.")?;
        }
        out.write_all(b"],\"sagas\":")
            .map_err(|_| "Export de staging inaccessible.")?;
        serde_json::to_writer(&mut *out, &saga_export(&self.personal, self)?)
            .map_err(|_| "Export invalide.")?;
        out.write_all(b",\"personal\":[")
            .map_err(|_| "Export de staging inaccessible.")?;
        for (index, profile) in profiles.iter().enumerate() {
            if index != 0 {
                out.write_all(b",")
                    .map_err(|_| "Export de staging inaccessible.")?;
            }
            serde_json::to_writer(&mut *out, &self.export_profile(profile)?)
                .map_err(|_| "Export invalide.")?;
        }
        out.write_all(b"],\"leases\":[]}")
            .map_err(|_| "Export de staging inaccessible.")?;
        tx.commit().map_err(db_error)?;
        Ok(posters)
    }

    fn export_json_with_limit(&mut self, maximum: Option<usize>) -> Result<Value, String> {
        // Reserve the single SQLite writer while both legacy and personal rows
        // are read. This is a coherent, bounded compatibility export; callers
        // needing a larger archive must use the future staging/streaming API.
        self.ensure_identities(&self.catalogue.list()?)?;
        let tx =
            rusqlite::Transaction::new_unchecked(&self.personal, TransactionBehavior::Immediate)
                .map_err(db_error)?;
        let media = self.catalogue.list()?;
        let sagas = saga_export(&self.personal, self)?;
        let profiles = self.profiles()?;
        let identities: Vec<Value> = media.iter().map(|m| Ok(json!({"id": self.stable_id(&m.id)?, "relative_path": m.id, "data": scrub_media(m)}))).collect::<Result<_, String>>()?;
        let personal = profiles
            .iter()
            .map(|p| self.export_profile(p))
            .collect::<Result<Vec<_>, _>>()?;
        let document = json!({"format_version": EXPORT_FORMAT_VERSION, "export_id": Uuid::new_v4().to_string(), "profiles": profiles, "media": identities, "sagas": sagas, "personal": personal, "leases": []});
        if maximum.is_some_and(|maximum| {
            serde_json::to_vec(&document)
                .map(|value| value.len() > maximum)
                .unwrap_or(true)
        }) {
            return Err(
                "Export trop volumineux pour l’API mémoire ; utilisez l’export de staging.".into(),
            );
        }
        tx.commit().map_err(db_error)?;
        Ok(document)
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

    /// Import application is intentionally disabled until the administrator layer
    /// supplies a complete, injective profile/root mapping and a reviewed conflict
    /// plan covering common metadata, sagas, and every personal record. Preview is
    /// read-only; callers must not treat it as authorization to mutate a library.
    pub fn import_prepared_json(
        &mut self,
        document: &Value,
        _profile_map: &std::collections::HashMap<String, String>,
    ) -> Result<bool, String> {
        self.preview_import(document)?;
        Err("Application d’import indisponible : préparez un plan complet de correspondances et conflits.".into())
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
        insert_identities(&tx, media)?;
        tx.execute("INSERT OR IGNORE INTO cc_media_revision(stable_id,revision) SELECT stable_id,1 FROM cc_media_identity", []).map_err(db_error)?;
        tx.commit().map_err(db_error)
    }
    fn stable_id(&self, legacy: &str) -> Result<String, String> {
        self.personal
            .query_row(
                "SELECT stable_id FROM cc_media_identity WHERE legacy_id=?1",
                [legacy],
                |r| r.get(0),
            )
            .map_err(media_lookup_error)
    }
    fn legacy_id(&self, stable: &str) -> Result<String, String> {
        self.personal
            .query_row(
                "SELECT legacy_id FROM cc_media_identity WHERE stable_id=?1",
                [stable],
                |r| r.get(0),
            )
            .map_err(media_lookup_error)
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
        // Legacy catalogue stores filesystem paths in `poster`. They are never
        // transport URLs: suppress absolute/local values until artwork is served
        // through the controlled artwork endpoint.
        // The file location/provider URL is storage-private.  The client gets a
        // stable opaque endpoint only; the handler resolves and validates it.
        if media.poster.is_some() {
            media.poster = Some(format!(
                "/api/v1/profiles/{profile}/media/{}/artwork",
                media.id
            ));
        }
        Ok(media)
    }
    pub fn artwork_path(&mut self, profile: &str, stable: &str) -> Result<PathBuf, String> {
        self.require_profile(profile)?;
        let legacy = self.legacy_id(stable)?;
        let media = self.catalogue.get(&legacy)?;
        let poster = media.poster.ok_or("Affiche introuvable.")?;
        if !Path::new(&poster).is_absolute() || poster.starts_with("file:") {
            return Err("Affiche introuvable.".into());
        }
        Ok(PathBuf::from(poster))
    }
    fn playlist_members(&self, id: &str) -> Result<Vec<String>, String> {
        playlist_members(&self.personal, id)
    }
    fn export_profile(&self, p: &Profile) -> Result<Value, String> {
        let prefs = self.preferences(&p.id)?;
        let mut speeds = Vec::new();
        let mut speed_query = self.personal.prepare("SELECT series_key,speed FROM cc_series_speed WHERE profile_id=?1 ORDER BY series_key").map_err(db_error)?;
        for row in speed_query
            .query_map([&p.id], |r| {
                Ok(json!({"series_key": r.get::<_,String>(0)?, "speed":r.get::<_,f64>(1)?}))
            })
            .map_err(db_error)?
        {
            speeds.push(row.map_err(db_error)?);
        }
        let mut states = Vec::new();
        let mut q=self.personal.prepare("SELECT stable_media_id,position_ms,final_watched_ms,watched,rating FROM cc_profile_state WHERE profile_id=?1").map_err(db_error)?;
        for r in q.query_map([&p.id],|r|Ok(json!({"media_id":r.get::<_,String>(0)?,"position_ms":r.get::<_,i64>(1)?,"final_watched_ms":r.get::<_,i64>(2)?,"watched":r.get::<_,bool>(3)?,"rating":r.get::<_,Option<i64>>(4)?}))).map_err(db_error)? { states.push(r.map_err(db_error)?); }
        Ok(
            json!({"profile_id":p.id,"preferences":prefs,"speeds":speeds,"states":states,"playlists":self.export_playlists(&p.id)?}),
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

fn add_column_if_missing(
    connection: &Connection,
    table: &str,
    definition: &str,
) -> Result<(), String> {
    let column = definition
        .split_whitespace()
        .next()
        .ok_or("Migration invalide.")?;
    let mut stmt = connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(db_error)?;
    let present = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?
        .iter()
        .any(|name| name == column);
    if !present {
        connection
            .execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {definition}"))
            .map_err(db_error)?;
    }
    Ok(())
}

fn insert_identities(tx: &rusqlite::Transaction<'_>, media: &[Media]) -> Result<(), String> {
    for m in media {
        tx.execute("INSERT OR IGNORE INTO cc_media_identity(stable_id,legacy_id,relative_path) VALUES(?1,?2,?3)",params![Uuid::new_v4().to_string(),m.id,m.id]).map_err(db_error)?;
    }
    Ok(())
}
fn db_error(_: rusqlite::Error) -> String {
    "Erreur de base locale. Vos données n’ont pas été remplacées.".into()
}
fn media_lookup_error(error: rusqlite::Error) -> String {
    if matches!(error, rusqlite::Error::QueryReturnedNoRows) {
        "Vidéo introuvable dans le catalogue.".into()
    } else {
        db_error(error)
    }
}
fn profile_collision(error: rusqlite::Error) -> String {
    if matches!(&error, rusqlite::Error::SqliteFailure(code, _) if code.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE)
    {
        "Ce nom de profil est déjà utilisé.".into()
    } else {
        db_error(error)
    }
}
pub(crate) fn valid_profile_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() || name.len() > 80 || name.chars().any(|c| c.is_control()) {
        Err("Nom de profil invalide.".into())
    } else {
        Ok(name.into())
    }
}
pub(crate) fn valid_collection_name(name: &str) -> Result<String, String> {
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
    let rating = match rating {
        Some(value) => value,
        None => old.rating,
    };
    tx.execute("INSERT INTO cc_profile_state(profile_id,stable_media_id,position_ms,final_watched_ms,watched,rating) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(profile_id,stable_media_id) DO UPDATE SET position_ms=excluded.position_ms,final_watched_ms=excluded.final_watched_ms,watched=excluded.watched,rating=excluded.rating",params![p,m,position.unwrap_or(old.position_ms) as i64,final_ms.flatten().unwrap_or(old.final_watched_ms) as i64,watched.unwrap_or(old.watched),rating]).map_err(db_error)?;
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
fn write_json_array<W: std::io::Write, T>(
    out: &mut W,
    values: impl Iterator<Item = T>,
    mut write: impl FnMut(&mut W, T) -> Result<(), String>,
) -> Result<(), String> {
    out.write_all(b"[")
        .map_err(|_| "Export de staging inaccessible.")?;
    for (index, value) in values.enumerate() {
        if index != 0 {
            out.write_all(b",")
                .map_err(|_| "Export de staging inaccessible.")?;
        }
        write(out, value)?;
    }
    out.write_all(b"]")
        .map_err(|_| "Export de staging inaccessible.".to_string())
}

fn saga_export(connection: &Connection, store: &Store) -> Result<Vec<Collection>, String> {
    let mut collections = connection
        .prepare("SELECT id,name FROM collections WHERE kind='saga' ORDER BY name,id")
        .map_err(db_error)?;
    let rows = collections
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(db_error)?;
    let mut result = Vec::new();
    for row in rows {
        let (id, name) = row.map_err(db_error)?;
        let mut members = connection
            .prepare(
                "SELECT media_id FROM collection_members WHERE collection_id=?1 ORDER BY rowid",
            )
            .map_err(db_error)?;
        let legacy = members
            .query_map([id], |row| row.get::<_, String>(0))
            .map_err(db_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db_error)?;
        result.push(Collection {
            id: id.to_string(),
            name,
            kind: "saga".into(),
            media_ids: legacy
                .iter()
                .map(|id| store.stable_id(id))
                .collect::<Result<_, _>>()?,
            poster: None,
        });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::tempdir;
    #[test]
    fn failed_identity_extension_rolls_back_catalogue_scan_marker_and_mapping() {
        let mut s = store();
        let before = serde_json::to_value(s.catalogue.list().unwrap()).unwrap();
        let marker: String = s
            .personal
            .query_row(
                "SELECT value FROM settings WHERE key='last_scan'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let identities: i64 = s
            .personal
            .query_row("SELECT COUNT(*) FROM cc_media_identity", [], |r| r.get(0))
            .unwrap();
        s.personal.execute_batch("CREATE TRIGGER fail_identity BEFORE INSERT ON cc_media_identity WHEN NEW.legacy_id='Films/C.mkv' BEGIN SELECT RAISE(ABORT,'injected identity failure'); END;").unwrap();
        assert!(
            s.snapshot(vec![media("Films/C.mkv", "/nas/Films/C.mkv")])
                .is_err()
        );
        assert_eq!(
            serde_json::to_value(s.catalogue.list().unwrap()).unwrap(),
            before
        );
        assert_eq!(
            s.personal
                .query_row(
                    "SELECT value FROM settings WHERE key='last_scan'",
                    [],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
            marker
        );
        assert_eq!(
            s.personal
                .query_row("SELECT COUNT(*) FROM cc_media_identity", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            identities
        );
    }
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
    fn validated_slow_playback_credit_is_real_time_not_video_duration() {
        let mut s = store();
        let p = s.create_profile("Slow").unwrap();
        let id = s.catalog(&p.id).unwrap()[0].id.clone();
        let event = Progress {
            session_id: "session".into(),
            generation: 1,
            media_id: id.clone(),
            media_revision: 1,
            sequence: 1,
            position_ms: 100_000,
            duration_ms: 100_000,
            speed: 0.5,
            final_elapsed_ms: 20_000,
            playing: true,
            buffering: false,
            seek: false,
        };
        assert!(
            s.save_validated_progress(&p.id, &id, &event)
                .unwrap()
                .watched
        );
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
    fn playlist_duplicate_rename_is_conflict_and_does_not_change_revision_or_order() {
        let mut s = store();
        let a = s.create_profile("A").unwrap();
        let b = s.create_profile("B").unwrap();
        let ids: Vec<_> = s
            .catalog(&a.id)
            .unwrap()
            .into_iter()
            .map(|m| m.id)
            .collect();
        let first = s
            .create_playlist_with_members(&a.id, "Première", &ids)
            .unwrap();
        let second = s
            .create_playlist_with_members(&a.id, "Deuxième", &[])
            .unwrap();
        assert_eq!(
            s.rename_playlist_at_revision(&a.id, &second.id, "Première", second.revision)
                .unwrap_err(),
            "PLAYLIST_NAME_CONFLICT"
        );
        let after = s.playlist(&a.id, &second.id).unwrap();
        assert_eq!(after.name, "Deuxième");
        assert_eq!(after.revision, second.revision);
        assert_eq!(s.playlist(&a.id, &first.id).unwrap().media_ids, ids);
        let other = s
            .create_playlist_with_members(&b.id, "Première", &[])
            .unwrap();
        assert_eq!(other.name, "Première");
    }
    #[test]
    fn saga_boundary_roundtrips_scanned_stable_ids_without_touching_personal_state() {
        let mut s = store();
        let p = s.create_profile("Saga").unwrap();
        let ids: Vec<_> = s
            .catalog(&p.id)
            .unwrap()
            .into_iter()
            .map(|m| m.id)
            .collect();
        // Snapshot stores relative legacy IDs, whereas all public APIs below use UUIDs.
        assert!(
            ids.iter()
                .all(|id| id != "Films/A.mkv" && id != "Films/B.mkv")
        );
        s.set_watched(&p.id, &ids[0], true).unwrap();
        s.set_rating(&p.id, &ids[0], Some(4), "video").unwrap();
        let saga = s.create_saga("Saga stable").unwrap();
        let saga = s
            .saga_members_at_revision(&saga.id, &ids, saga.revision, true)
            .unwrap();
        assert_eq!(saga.media_ids, ids);
        assert!(
            s.validate_playback_context(
                &p.id,
                &ids[0],
                &PlaybackContext {
                    kind: "saga".into(),
                    id: Some(saga.id.clone())
                }
            )
            .is_ok()
        );
        let before = saga.clone();
        assert!(
            s.saga_members_at_revision(&before.id, &[ids[0].clone()], before.revision - 1, false)
                .is_err()
        );
        assert_eq!(s.saga(&before.id).unwrap().media_ids, ids);
        let saga = s
            .reorder_saga_at_revision(
                &before.id,
                before.revision,
                &ids,
                &[ids[1].clone(), ids[0].clone()],
            )
            .unwrap();
        assert_eq!(saga.media_ids, vec![ids[1].clone(), ids[0].clone()]);
        let saga = s
            .saga_members_at_revision(&saga.id, &[ids[1].clone()], saga.revision, false)
            .unwrap();
        assert_eq!(saga.media_ids, vec![ids[0].clone()]);
        let view = s.media(&p.id, &ids[0]).unwrap();
        assert!(view.watched);
        assert_eq!(view.rating, Some(4));
    }
    #[test]
    fn media_cas_context_and_next_are_profile_bound() {
        let mut s = store();
        let p = s.create_profile("A").unwrap();
        let other = s.create_profile("B").unwrap();
        let ids: Vec<_> = s
            .catalog(&p.id)
            .unwrap()
            .into_iter()
            .map(|m| m.id)
            .collect();
        let revision = s.media_revision(&ids[0]).unwrap();
        let (changed, next_revision) = s
            .update_media_at_revision(
                &p.id,
                &ids[0],
                revision,
                &MediaPatch {
                    title: Some("Nouveau".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(changed.title, "Nouveau");
        assert!(
            s.update_media_at_revision(&p.id, &ids[0], revision, &MediaPatch::default())
                .is_err()
        );
        assert_eq!(next_revision, revision + 1);
        let list = s.create_collection(&p.id, "Liste", "playlist").unwrap();
        s.collection_member_add_bulk(&p.id, &list.id, &ids).unwrap();
        let context = PlaybackContext {
            kind: "playlist".into(),
            id: Some(list.id.clone()),
        };
        assert_eq!(
            s.next_video(&p.id, &ids[0], &context)
                .unwrap()
                .media
                .unwrap()
                .id,
            ids[1]
        );
        assert!(
            s.validate_playback_context(&other.id, &ids[0], &context)
                .is_err()
        );
    }
    #[test]
    fn group_ratings_only_touch_present_targeted_episodes() {
        let mut s = store();
        let p = s.create_profile("A").unwrap();
        let mut a = media("S/a.mkv", "/nas/S/a.mkv");
        a.category = "serie".into();
        a.series = Some("S".into());
        a.season = Some(1);
        a.episode = Some(1);
        let mut b = media("S/b.mkv", "/nas/S/b.mkv");
        b.category = "serie".into();
        b.series = Some("S".into());
        b.season = Some(2);
        b.episode = Some(1);
        s.snapshot(vec![a, b]).unwrap();
        let ids: Vec<_> = s
            .catalog(&p.id)
            .unwrap()
            .into_iter()
            .filter(|m| m.series.as_deref() == Some("S"))
            .map(|m| m.id)
            .collect();
        s.set_rating(&p.id, &ids[0], Some(4), "season").unwrap();
        assert_eq!(s.media(&p.id, &ids[0]).unwrap().rating, Some(4));
        assert_eq!(s.media(&p.id, &ids[1]).unwrap().rating, None);
    }
    #[test]
    fn watch_threshold_and_manual_reset_preserve_resume() {
        let mut s = store();
        let p = s.create_profile("A").unwrap();
        let id = s.catalog(&p.id).unwrap()[0].id.clone();
        assert!(s.save_progress(&p.id, &id, 50, 9999, 100000).is_err());
        let progress = |sequence, final_elapsed_ms| Progress {
            media_revision: 1,
            session_id: "session".into(),
            generation: 1,
            media_id: id.clone(),
            sequence,
            position_ms: 95_000,
            duration_ms: 100_000,
            speed: 1.0,
            final_elapsed_ms,
            playing: true,
            buffering: false,
            seek: false,
        };
        assert!(
            !s.save_validated_progress(&p.id, &id, &progress(1, 9_999))
                .unwrap()
                .watched
        );
        assert!(
            s.save_validated_progress(&p.id, &id, &progress(2, 1))
                .unwrap()
                .watched
        );
        assert!(
            s.save_validated_progress(&p.id, &id, &progress(2, 1))
                .is_err()
        );
        s.set_watched(&p.id, &id, false).unwrap();
        let state = state(&s.personal, &p.id, &id).unwrap();
        assert!(!state.watched);
        assert_eq!(state.final_watched_ms, 0);
        assert_eq!(state.position_ms, 95_000);
    }

    #[test]
    fn ordered_nonfinal_pause_and_seek_events_persist_resume_without_credit() {
        let mut s = store();
        let p = s.create_profile("A").unwrap();
        let id = s.catalog(&p.id).unwrap()[0].id.clone();
        let event = |sequence, position_ms, playing, buffering, seek, final_elapsed_ms| Progress {
            media_revision: 1,
            session_id: "session".into(),
            generation: 1,
            media_id: id.clone(),
            sequence,
            position_ms,
            duration_ms: 100_000,
            speed: 1.0,
            final_elapsed_ms,
            playing,
            buffering,
            seek,
        };
        assert_eq!(
            s.save_validated_progress(&p.id, &id, &event(1, 10_000, true, false, false, 0))
                .unwrap()
                .position_ms,
            10_000
        );
        assert_eq!(
            s.save_validated_progress(&p.id, &id, &event(2, 12_000, false, false, false, 0))
                .unwrap()
                .position_ms,
            12_000
        );
        assert_eq!(
            s.save_validated_progress(&p.id, &id, &event(3, 20_000, true, false, true, 0))
                .unwrap()
                .position_ms,
            20_000
        );
        assert_eq!(
            s.save_validated_progress(&p.id, &id, &event(4, 20_000, false, false, false, 0))
                .unwrap()
                .final_watched_ms,
            0
        );
        assert!(
            s.save_validated_progress(&p.id, &id, &event(5, 20_000, false, false, false, 1))
                .is_err()
        );
        assert!(
            s.save_validated_progress(&p.id, &id, &event(4, 30_000, true, false, false, 0))
                .is_err()
        );
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
    fn mutable_import_is_disabled_until_a_complete_conflict_plan_exists() {
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
                .is_err()
        );
        let destination_id = destination.catalog(&destination_profile.id).unwrap()[0]
            .id
            .clone();
        assert_eq!(
            destination
                .media(&destination_profile.id, &destination_id)
                .unwrap()
                .rating,
            None
        );
    }

    #[test]
    fn catalog_epoch_advances_for_committed_changes_and_not_rollback() {
        let s = store();
        let before = s.catalog_revision().unwrap();
        let p = s.create_profile("Epoch").unwrap();
        assert!(s.catalog_revision().unwrap() > before);
        let after = s.catalog_revision().unwrap();
        let tx = s.personal.unchecked_transaction().unwrap();
        tx.execute(
            "UPDATE cc_profiles SET name='Rolled back' WHERE id=?1",
            [&p.id],
        )
        .unwrap();
        tx.rollback().unwrap();
        assert_eq!(s.catalog_revision().unwrap(), after);
    }

    #[test]
    fn rating_can_be_removed_and_playlist_removal_revisions_atomically() {
        let mut s = store();
        let p = s.create_profile("A").unwrap();
        let ids: Vec<_> = s
            .catalog(&p.id)
            .unwrap()
            .into_iter()
            .map(|m| m.id)
            .collect();
        s.set_rating(&p.id, &ids[0], Some(5), "video").unwrap();
        s.set_rating(&p.id, &ids[0], None, "video").unwrap();
        assert_eq!(s.media(&p.id, &ids[0]).unwrap().rating, None);
        let list = s.create_collection(&p.id, "Liste", "playlist").unwrap();
        s.collection_member_add_bulk(&p.id, &list.id, &ids).unwrap();
        s.collection_member_remove_at_revision(&p.id, &list.id, &ids[0], Some(2))
            .unwrap();
        assert!(
            s.collection_member_remove_at_revision(&p.id, &list.id, &ids[1], Some(2))
                .is_err()
        );
        assert_eq!(
            s.collections(&p.id)
                .unwrap()
                .into_iter()
                .find(|c| c.id == list.id)
                .unwrap()
                .media_ids,
            vec![ids[1].clone()]
        );
    }

    #[test]
    fn reassociation_requires_explicit_confirmation_and_keeps_personal_history() {
        let mut s = store();
        let p = s.create_profile("A").unwrap();
        let original = s.catalog(&p.id).unwrap()[0].id.clone();
        s.set_rating(&p.id, &original, Some(5), "video").unwrap();
        s.snapshot(vec![
            media("Films/Renommé.mkv", "/nas/Films/Renommé.mkv"),
            media("Films/B.mkv", "/nas/Films/B.mkv"),
        ])
        .unwrap();
        s.confirm_media_reassociation(&original, "Films/Renommé.mkv")
            .unwrap();
        assert_eq!(s.media(&p.id, &original).unwrap().rating, Some(5));
        assert_eq!(s.media(&p.id, &original).unwrap().path, "Films/Renommé.mkv");
    }

    #[test]
    fn catalogue_page_is_bounded_and_filters_in_sqlite() {
        let mut s = store();
        let p = s.create_profile("A").unwrap();
        let page = s
            .catalog_page(
                &p.id,
                &CatalogQuery {
                    limit: 1,
                    category: Some("film".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(page.items.len(), 1);
        assert!(page.has_more);
    }
}
