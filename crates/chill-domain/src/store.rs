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
    model::{Collection, Media, MediaPatch, SeasonProvider},
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

/// Legacy flat episode folders may use `001 Title.avi` without SxxExx metadata.
/// Resolve their existing source order without inventing a season or mutating
/// titles. Require one folder, explicit unique ordinals, and no numbered seasons.
fn flat_series_order(members: &[Media]) -> Option<Vec<&Media>> {
    let parent = Path::new(&members.first()?.path).parent()?;
    let mut numbers = HashSet::new();
    let mut ordered = Vec::with_capacity(members.len());
    for member in members {
        if member.season.is_some() || Path::new(&member.path).parent()? != parent {
            return None;
        }
        let number = member
            .episode
            .filter(|number| *number > 0)
            .map(|v| v as u32)
            .or_else(|| {
                let stem = Path::new(&member.path).file_stem()?.to_str()?;
                let digits = stem.bytes().take_while(u8::is_ascii_digit).count();
                if !(1..=4).contains(&digits) {
                    return None;
                }
                let rest = &stem[digits..];
                if !rest.is_empty() && !rest.starts_with([' ', '-', '_', '.']) {
                    return None;
                }
                let number = stem[..digits].parse::<u32>().ok()?;
                // A leading release year is not evidence of an episode ordinal.
                (number > 0 && !(digits == 4 && (1800..=2099).contains(&number))).then_some(number)
            })?;
        if !numbers.insert(number) {
            return None;
        }
        ordered.push((number, member));
    }
    ordered.sort_by_key(|(number, _)| *number);
    Some(ordered.into_iter().map(|(_, media)| media).collect())
}
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
    pub series_counters: std::collections::BTreeMap<String, SeriesCounters>,
}
#[derive(Debug, Clone, serde::Serialize)]
pub struct SeriesCounters {
    pub series_key: String,
    pub seasons: u64,
    pub episodes: u64,
    pub unwatched: u64,
}

/// Personal playlist view.  The revision is deliberately separate from the
/// common legacy `Collection`: clients must supply it for every mutation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlaylistView {
    pub id: String,
    pub name: String,
    pub revision: u64,
    pub media_ids: Vec<String>,
    #[serde(default)]
    pub poster: Option<String>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SagaView {
    pub id: String,
    pub name: String,
    pub revision: u64,
    pub media_ids: Vec<String>,
    pub poster: Option<String>,
}

/// A bounded, profile-scoped catalogue image suitable for a client-side mirror.
/// It contains relative media references only; it is never an operational backup.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncMedia {
    pub media: Media,
    pub revision: u64,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncSnapshot {
    pub items: Vec<SyncMedia>,
    pub sagas: Vec<SagaView>,
    pub playlists: Vec<PlaylistView>,
    pub series_revisions: std::collections::BTreeMap<String, u64>,
    /// Season data is part of the cache document, not duplicated on every
    /// episode. Keys are `tmdb:tv:<id>:season:<number>`.
    #[serde(default)]
    pub season_metadata: std::collections::BTreeMap<String, SeasonProvider>,
    #[serde(skip)]
    pub serialized_bytes: usize,
    #[serde(skip)]
    pub captured_revision: (u64, u64),
}

pub struct Store {
    pub(crate) catalogue: Database,
    pub(crate) personal: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self, String> {
        if path.exists() {
            let probe =
                Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                    .map_err(db_error)?;
            let version: u32 = probe
                .pragma_query_value(None, "user_version", |r| r.get(0))
                .map_err(db_error)?;
            if version > 3 {
                return Err("Schéma plus récent : restauration pré-migration requise pour revenir à cette version.".into());
            }
        }
        let catalogue = Database::open(path)?;
        let personal = Connection::open(path).map_err(db_error)?;
        personal
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(db_error)?;
        personal.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS cc_common_epoch (id INTEGER PRIMARY KEY CHECK(id=1), revision INTEGER NOT NULL);
             INSERT OR IGNORE INTO cc_common_epoch(id,revision) VALUES(1,0);
             CREATE TRIGGER IF NOT EXISTS cc_common_media_i AFTER INSERT ON media BEGIN UPDATE cc_common_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_common_media_u AFTER UPDATE ON media BEGIN UPDATE cc_common_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_common_media_d AFTER DELETE ON media BEGIN UPDATE cc_common_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TABLE IF NOT EXISTS cc_personal_epoch (profile_id TEXT PRIMARY KEY REFERENCES cc_profiles(id) ON DELETE CASCADE, revision INTEGER NOT NULL DEFAULT 0);
             CREATE TABLE IF NOT EXISTS cc_profiles (
                id TEXT PRIMARY KEY, name TEXT NOT NULL COLLATE NOCASE UNIQUE,
                revision INTEGER NOT NULL DEFAULT 1
             );
             CREATE TABLE IF NOT EXISTS cc_media_identity (
                stable_id TEXT PRIMARY KEY, legacy_id TEXT NOT NULL UNIQUE,
                relative_path TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS cc_series_group (
                legacy_id TEXT PRIMARY KEY,
                source_key TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS cc_media_revision (
                stable_id TEXT PRIMARY KEY REFERENCES cc_media_identity(stable_id) ON DELETE CASCADE,
                revision INTEGER NOT NULL DEFAULT 1
             );
             -- Provider data shared by every episode of a selected TV season.
             -- It is keyed by TMDB id, not title, so homonymous series cannot
             -- overwrite each other.
             CREATE TABLE IF NOT EXISTS cc_season_provider (
                tv_id INTEGER NOT NULL CHECK(tv_id>0),
                season INTEGER NOT NULL CHECK(season>=0),
                data TEXT NOT NULL,
                updated_at INTEGER NOT NULL,
                PRIMARY KEY(tv_id,season)
             );
             CREATE TABLE IF NOT EXISTS cc_media_fingerprint (
                stable_id TEXT PRIMARY KEY REFERENCES cc_media_identity(stable_id) ON DELETE CASCADE,
                byte_size INTEGER CHECK(byte_size > 0),
                partial_sha256 TEXT CHECK(partial_sha256 IS NULL OR length(partial_sha256)=64),
                full_sha256 TEXT CHECK(full_sha256 IS NULL OR length(full_sha256)=64),
                source_path TEXT,
                file_version_token TEXT CHECK(file_version_token IS NULL OR length(file_version_token)=64),
                state TEXT NOT NULL CHECK(state IN ('queued','running','complete','failed')),
                requested_kind TEXT NOT NULL CHECK(requested_kind IN ('partial','full'))
             );
             CREATE INDEX IF NOT EXISTS cc_media_fingerprint_match
                ON cc_media_fingerprint(byte_size,partial_sha256,full_sha256);
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
             -- Exact durations observed during lease-validated playback only.
             -- Separate from catalogue sync: playhead updates must not rebuild it.
             CREATE TABLE IF NOT EXISTS cc_resume_measurement (
                profile_id TEXT NOT NULL,
                stable_media_id TEXT NOT NULL,
                duration_ms INTEGER NOT NULL CHECK(duration_ms>0),
                PRIMARY KEY(profile_id,stable_media_id),
                FOREIGN KEY(profile_id,stable_media_id)
                  REFERENCES cc_profile_state(profile_id,stable_media_id) ON DELETE CASCADE ON UPDATE CASCADE
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
             CREATE TABLE IF NOT EXISTS cc_playlist_artwork (
                playlist_id TEXT PRIMARY KEY REFERENCES cc_playlists(id) ON DELETE CASCADE,
                poster TEXT NOT NULL
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
             CREATE TRIGGER IF NOT EXISTS cc_epoch_playlist_artwork_i AFTER INSERT ON cc_playlist_artwork BEGIN UPDATE cc_catalog_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_epoch_playlist_artwork_u AFTER UPDATE ON cc_playlist_artwork BEGIN UPDATE cc_catalog_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_epoch_playlist_artwork_d AFTER DELETE ON cc_playlist_artwork BEGIN UPDATE cc_catalog_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_epoch_collections_i AFTER INSERT ON collections BEGIN UPDATE cc_catalog_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_epoch_collections_u AFTER UPDATE ON collections BEGIN UPDATE cc_catalog_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_epoch_collections_d AFTER DELETE ON collections BEGIN UPDATE cc_catalog_epoch SET revision=revision+1 WHERE id=1; END;
             PRAGMA user_version=3;"
        ).map_err(db_error)?;
        // Catalogue mirrors do not include playhead, language or speed. Such
        // writes must not invalidate every cached catalogue in the household.
        personal.execute_batch("CREATE TABLE IF NOT EXISTS cc_sync_personal_epoch(profile_id TEXT PRIMARY KEY REFERENCES cc_profiles(id) ON DELETE CASCADE,revision INTEGER NOT NULL DEFAULT 0);
        INSERT OR IGNORE INTO cc_sync_personal_epoch(profile_id) SELECT id FROM cc_profiles;
        CREATE TRIGGER IF NOT EXISTS cc_sync_profile_i AFTER INSERT ON cc_profiles BEGIN INSERT INTO cc_sync_personal_epoch(profile_id) VALUES(NEW.id); END;
        CREATE TRIGGER IF NOT EXISTS cc_sync_state_i AFTER INSERT ON cc_profile_state WHEN NEW.watched<>0 OR NEW.rating IS NOT NULL BEGIN UPDATE cc_sync_personal_epoch SET revision=revision+1 WHERE profile_id=NEW.profile_id; END;
        CREATE TRIGGER IF NOT EXISTS cc_sync_state_u AFTER UPDATE OF watched,rating ON cc_profile_state WHEN NEW.watched IS NOT OLD.watched OR NEW.rating IS NOT OLD.rating BEGIN UPDATE cc_sync_personal_epoch SET revision=revision+1 WHERE profile_id=NEW.profile_id; END;
        CREATE TRIGGER IF NOT EXISTS cc_sync_state_d AFTER DELETE ON cc_profile_state WHEN OLD.watched<>0 OR OLD.rating IS NOT NULL BEGIN UPDATE cc_sync_personal_epoch SET revision=revision+1 WHERE profile_id=OLD.profile_id; END;
        CREATE TRIGGER IF NOT EXISTS cc_sync_playlist_i AFTER INSERT ON cc_playlists BEGIN UPDATE cc_sync_personal_epoch SET revision=revision+1 WHERE profile_id=NEW.profile_id; END;
        CREATE TRIGGER IF NOT EXISTS cc_sync_playlist_u AFTER UPDATE ON cc_playlists BEGIN UPDATE cc_sync_personal_epoch SET revision=revision+1 WHERE profile_id=NEW.profile_id; END;
        CREATE TRIGGER IF NOT EXISTS cc_sync_playlist_d AFTER DELETE ON cc_playlists BEGIN UPDATE cc_sync_personal_epoch SET revision=revision+1 WHERE profile_id=OLD.profile_id; END;
        CREATE TRIGGER IF NOT EXISTS cc_sync_member_i AFTER INSERT ON cc_playlist_members BEGIN UPDATE cc_sync_personal_epoch SET revision=revision+1 WHERE profile_id=(SELECT profile_id FROM cc_playlists WHERE id=NEW.playlist_id); END;
        CREATE TRIGGER IF NOT EXISTS cc_sync_member_u AFTER UPDATE ON cc_playlist_members BEGIN UPDATE cc_sync_personal_epoch SET revision=revision+1 WHERE profile_id=(SELECT profile_id FROM cc_playlists WHERE id=NEW.playlist_id); END;
        CREATE TRIGGER IF NOT EXISTS cc_sync_member_d AFTER DELETE ON cc_playlist_members BEGIN UPDATE cc_sync_personal_epoch SET revision=revision+1 WHERE profile_id=(SELECT profile_id FROM cc_playlists WHERE id=OLD.playlist_id); END;").map_err(db_error)?;
        // AI plans have two independent stale tokens.  Keep their maintenance in
        // SQLite so mutations from every Store connection are visible atomically.
        personal.execute_batch(
            "INSERT OR IGNORE INTO cc_personal_epoch(profile_id,revision) SELECT id,0 FROM cc_profiles;
             CREATE TRIGGER IF NOT EXISTS cc_personal_profile_i AFTER INSERT ON cc_profiles BEGIN INSERT OR IGNORE INTO cc_personal_epoch(profile_id,revision) VALUES(NEW.id,0); END;
             CREATE TRIGGER IF NOT EXISTS cc_personal_profile_u AFTER UPDATE OF name ON cc_profiles BEGIN UPDATE cc_personal_epoch SET revision=revision+1 WHERE profile_id=NEW.id; END;
             CREATE TRIGGER IF NOT EXISTS cc_personal_state_i AFTER INSERT ON cc_profile_state BEGIN UPDATE cc_personal_epoch SET revision=revision+1 WHERE profile_id=NEW.profile_id; END;
             CREATE TRIGGER IF NOT EXISTS cc_personal_state_u AFTER UPDATE ON cc_profile_state BEGIN UPDATE cc_personal_epoch SET revision=revision+1 WHERE profile_id=NEW.profile_id; END;
             CREATE TRIGGER IF NOT EXISTS cc_personal_state_d AFTER DELETE ON cc_profile_state BEGIN UPDATE cc_personal_epoch SET revision=revision+1 WHERE profile_id=OLD.profile_id; END;
             CREATE TRIGGER IF NOT EXISTS cc_personal_preferences_i AFTER INSERT ON cc_preferences BEGIN UPDATE cc_personal_epoch SET revision=revision+1 WHERE profile_id=NEW.profile_id; END;
             CREATE TRIGGER IF NOT EXISTS cc_personal_preferences_u AFTER UPDATE ON cc_preferences BEGIN UPDATE cc_personal_epoch SET revision=revision+1 WHERE profile_id=NEW.profile_id; END;
             CREATE TRIGGER IF NOT EXISTS cc_personal_preferences_d AFTER DELETE ON cc_preferences BEGIN UPDATE cc_personal_epoch SET revision=revision+1 WHERE profile_id=OLD.profile_id; END;
             CREATE TRIGGER IF NOT EXISTS cc_personal_speed_i AFTER INSERT ON cc_series_speed BEGIN UPDATE cc_personal_epoch SET revision=revision+1 WHERE profile_id=NEW.profile_id; END;
             CREATE TRIGGER IF NOT EXISTS cc_personal_speed_u AFTER UPDATE ON cc_series_speed BEGIN UPDATE cc_personal_epoch SET revision=revision+1 WHERE profile_id=NEW.profile_id; END;
             CREATE TRIGGER IF NOT EXISTS cc_personal_speed_d AFTER DELETE ON cc_series_speed BEGIN UPDATE cc_personal_epoch SET revision=revision+1 WHERE profile_id=OLD.profile_id; END;
             CREATE TRIGGER IF NOT EXISTS cc_personal_playlist_i AFTER INSERT ON cc_playlists BEGIN UPDATE cc_personal_epoch SET revision=revision+1 WHERE profile_id=NEW.profile_id; END;
             CREATE TRIGGER IF NOT EXISTS cc_personal_playlist_u AFTER UPDATE ON cc_playlists BEGIN UPDATE cc_personal_epoch SET revision=revision+1 WHERE profile_id=NEW.profile_id; END;
             CREATE TRIGGER IF NOT EXISTS cc_personal_playlist_d AFTER DELETE ON cc_playlists BEGIN UPDATE cc_personal_epoch SET revision=revision+1 WHERE profile_id=OLD.profile_id; END;
             CREATE TRIGGER IF NOT EXISTS cc_personal_member_i AFTER INSERT ON cc_playlist_members BEGIN UPDATE cc_personal_epoch SET revision=revision+1 WHERE profile_id=(SELECT profile_id FROM cc_playlists WHERE id=NEW.playlist_id); END;
             CREATE TRIGGER IF NOT EXISTS cc_personal_member_u AFTER UPDATE ON cc_playlist_members BEGIN UPDATE cc_personal_epoch SET revision=revision+1 WHERE profile_id=(SELECT profile_id FROM cc_playlists WHERE id=NEW.playlist_id); END;
             CREATE TRIGGER IF NOT EXISTS cc_personal_member_d AFTER DELETE ON cc_playlist_members BEGIN UPDATE cc_personal_epoch SET revision=revision+1 WHERE profile_id=(SELECT profile_id FROM cc_playlists WHERE id=OLD.playlist_id); END;
             CREATE TRIGGER IF NOT EXISTS cc_personal_playlist_artwork_i AFTER INSERT ON cc_playlist_artwork BEGIN UPDATE cc_personal_epoch SET revision=revision+1 WHERE profile_id=(SELECT profile_id FROM cc_playlists WHERE id=NEW.playlist_id); END;
             CREATE TRIGGER IF NOT EXISTS cc_personal_playlist_artwork_u AFTER UPDATE ON cc_playlist_artwork BEGIN UPDATE cc_personal_epoch SET revision=revision+1 WHERE profile_id=(SELECT profile_id FROM cc_playlists WHERE id=NEW.playlist_id); END;
             CREATE TRIGGER IF NOT EXISTS cc_personal_playlist_artwork_d AFTER DELETE ON cc_playlist_artwork BEGIN UPDATE cc_personal_epoch SET revision=revision+1 WHERE profile_id=(SELECT profile_id FROM cc_playlists WHERE id=OLD.playlist_id); END;
             CREATE TRIGGER IF NOT EXISTS cc_common_collections_i AFTER INSERT ON collections BEGIN UPDATE cc_common_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_common_collections_u AFTER UPDATE ON collections BEGIN UPDATE cc_common_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_common_collections_d AFTER DELETE ON collections BEGIN UPDATE cc_common_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_common_members_i AFTER INSERT ON collection_members BEGIN UPDATE cc_common_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_common_members_u AFTER UPDATE ON collection_members BEGIN UPDATE cc_common_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_common_members_d AFTER DELETE ON collection_members BEGIN UPDATE cc_common_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_common_positions_i AFTER INSERT ON collection_positions BEGIN UPDATE cc_common_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_common_positions_u AFTER UPDATE ON collection_positions BEGIN UPDATE cc_common_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_common_positions_d AFTER DELETE ON collection_positions BEGIN UPDATE cc_common_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_common_artwork_i AFTER INSERT ON collection_artwork BEGIN UPDATE cc_common_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_common_artwork_u AFTER UPDATE ON collection_artwork BEGIN UPDATE cc_common_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_common_artwork_d AFTER DELETE ON collection_artwork BEGIN UPDATE cc_common_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_common_revisions_i AFTER INSERT ON collection_revisions BEGIN UPDATE cc_common_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_common_revisions_u AFTER UPDATE ON collection_revisions BEGIN UPDATE cc_common_epoch SET revision=revision+1 WHERE id=1; END;
             CREATE TRIGGER IF NOT EXISTS cc_common_revisions_d AFTER DELETE ON collection_revisions BEGIN UPDATE cc_common_epoch SET revision=revision+1 WHERE id=1; END;"
        ).map_err(db_error)?;
        // Existing development databases may predate ordered progress guards.
        add_column_if_missing(&personal, "cc_profile_state", "last_session_id TEXT")?;
        add_column_if_missing(&personal, "cc_profile_state", "last_generation INTEGER")?;
        add_column_if_missing(&personal, "cc_profile_state", "last_sequence INTEGER")?;
        add_column_if_missing(&personal, "cc_media_fingerprint", "source_path TEXT")?;
        add_column_if_missing(
            &personal,
            "cc_media_fingerprint",
            "file_version_token TEXT CHECK(file_version_token IS NULL OR length(file_version_token)=64)",
        )?;
        // A content hash captured before physical-version provenance existed is
        // not sufficient for automatic reassociation. Keep its partial evidence,
        // but conservatively remove only the unproven full strength.
        personal
            .execute(
                "UPDATE cc_media_fingerprint SET full_sha256=NULL
                 WHERE full_sha256 IS NOT NULL AND file_version_token IS NULL",
                [],
            )
            .map_err(db_error)?;
        // apply_reassociation copies the target evidence while both rows still
        // exist. Keep the private invalidation token in that same transaction
        // without exposing it through the reassociation/API contracts.
        personal
            .execute_batch(
                "CREATE TRIGGER IF NOT EXISTS cc_fingerprint_version_reassociation
             AFTER UPDATE OF source_path ON cc_media_fingerprint
             WHEN NEW.source_path IS NOT OLD.source_path AND EXISTS(
               SELECT 1 FROM cc_media_fingerprint other
               JOIN cc_media_identity oi ON oi.stable_id=other.stable_id
               JOIN media om ON om.id=oi.legacy_id
               WHERE other.stable_id<>NEW.stable_id AND other.source_path=NEW.source_path
                 AND other.source_path=json_extract(om.data,'$.path')
             )
             BEGIN
               UPDATE cc_media_fingerprint SET file_version_token=(
                 SELECT other.file_version_token FROM cc_media_fingerprint other
                 JOIN cc_media_identity oi ON oi.stable_id=other.stable_id
                 JOIN media om ON om.id=oi.legacy_id
                 WHERE other.stable_id<>NEW.stable_id AND other.source_path=NEW.source_path
                   AND other.source_path=json_extract(om.data,'$.path')
                 ORDER BY other.stable_id LIMIT 1
               ) WHERE stable_id=NEW.stable_id;
             END;",
            )
            .map_err(db_error)?;
        // The epoch guards the entire imported state, including writes from a
        // second connection (scanner). Every trigger is transactional.
        for table in [
            "cc_media_identity",
            "cc_media_revision",
            "cc_preferences",
            "cc_series_speed",
            "cc_season_provider",
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

    pub fn open_export_snapshot(path: &Path) -> Result<Self, String> {
        let personal =
            Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(db_error)?;
        let version: u32 = personal
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(db_error)?;
        if version != 3 {
            return Err("Snapshot incompatible.".into());
        }
        Ok(Self {
            catalogue: Database::open_readonly(path)?,
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
    pub fn create_profile_with_preferences(
        &self,
        name: &str,
        preferences: Option<Preferences>,
    ) -> Result<Profile, String> {
        if let Some(p) = &preferences {
            p.validate()?;
        }
        let tx = self.personal.unchecked_transaction().map_err(db_error)?;
        let profile = self.create_profile(name)?;
        if let Some(preferences) = preferences {
            self.save_preferences(&profile.id, &preferences)?;
        }
        tx.commit().map_err(db_error)?;
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
    /// Stable preference identity, independent of the mutable display label.
    pub fn series_identity(&self, stable: &str) -> Result<String, String> {
        let raw = self.catalogue.get(&self.legacy_id(stable)?)?;
        if raw.category != "serie" {
            return Err("Cette vidéo n’est pas une série.".into());
        }
        if let Some(id) = raw.tmdb_id {
            return Ok(format!("tmdb:tv:{id}"));
        }
        let members = self.catalogue.series_members_for(&raw)?;
        let anchor = members
            .iter()
            .min_by(|a, b| a.added_at.cmp(&b.added_at).then_with(|| a.id.cmp(&b.id)))
            .ok_or("Série introuvable.")?;
        Ok(format!("media:{}", self.stable_id(&anchor.id)?))
    }
    pub fn series_playback_speed(&mut self, profile: &str, stable: &str) -> Result<f64, String> {
        self.require_profile(profile)?;
        let media = self.media(profile, stable)?;
        if media.category != "serie" {
            return Ok(1.0);
        }
        let key = self.series_identity(stable)?;
        let speed: Option<f64> = self
            .personal
            .query_row(
                "SELECT speed FROM cc_series_speed WHERE profile_id=?1 AND series_key=?2",
                params![profile, key],
                |row| row.get(0),
            )
            .optional()
            .map_err(db_error)?;
        if let Some(speed) = speed {
            return Ok(speed);
        }
        let name = media.series.as_deref().unwrap_or(&media.title);
        // Legacy name-only preferences are usable only for a unique show.
        if self.series(profile, name).is_ok() {
            self.series_speed(profile, name)
        } else {
            Ok(1.0)
        }
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
        Ok(CatalogPage {
            items,
            has_more,
            series_counters: Default::default(),
        })
    }

    pub fn media(&mut self, profile_id: &str, stable_id: &str) -> Result<Media, String> {
        self.require_profile(profile_id)?;
        let legacy = self.legacy_id(stable_id)?;
        self.profile_view(profile_id, self.catalogue.get(&legacy)?)
    }

    /// Record a leaf file that disappeared after it was already indexed.  The
    /// caller must have verified that exact condition at the descriptor layer;
    /// this method never probes a path or starts a scan.  Keeping the row lets
    /// the unavailable-library workflow preserve its history and later restore
    /// it when a normal scan rediscovers the file.
    pub fn mark_media_unavailable_at_path(
        &mut self,
        profile_id: &str,
        stable_id: &str,
        expected_path: &Path,
    ) -> Result<bool, String> {
        self.require_profile(profile_id)?;
        let legacy = self.legacy_id(stable_id)?;
        let expected_path = expected_path.to_string_lossy();
        // The common `media` table and the personal revision table share this
        // SQLite file.  Do both writes in one IMMEDIATE transaction: a failed
        // revision bump must never leave an unversioned unavailable flag.
        let tx = rusqlite::Transaction::new_unchecked(
            &self.personal,
            rusqlite::TransactionBehavior::Immediate,
        )
        .map_err(db_error)?;
        let data: String = tx
            .query_row("SELECT data FROM media WHERE id=?1", [&legacy], |row| {
                row.get(0)
            })
            .map_err(db_error)?;
        let mut raw: Media = serde_json::from_str(&data)
            .map_err(|_| "Catalogue local endommagé : aucune donnée remplacée.".to_owned())?;
        if raw.path != expected_path || !raw.available {
            tx.commit().map_err(db_error)?;
            return Ok(false);
        }
        raw.available = false;
        tx.execute(
            "UPDATE media SET data=?1 WHERE id=?2",
            params![
                serde_json::to_string(&raw).map_err(|_| "Métadonnées invalides.")?,
                legacy
            ],
        )
        .map_err(db_error)?;
        if tx
            .execute(
                "UPDATE cc_media_revision SET revision=revision+1 WHERE stable_id=?1",
                [stable_id],
            )
            .map_err(db_error)?
            != 1
        {
            return Err("Vidéo introuvable dans le catalogue.".into());
        }
        tx.commit().map_err(db_error)?;
        Ok(true)
    }

    pub fn series(&mut self, profile_id: &str, name: &str) -> Result<Vec<Media>, String> {
        self.require_profile(profile_id)?;
        if name.is_empty() || name.len() > 500 {
            return Err("Série invalide.".into());
        }
        if let Some(anchor) = name.strip_prefix("media:") {
            let raw = self.catalogue.get(&self.legacy_id(anchor)?)?;
            let mut members = self
                .catalogue
                .series_members_for(&raw)?
                .into_iter()
                .map(|m| self.profile_view(profile_id, m))
                .collect::<Result<Vec<_>, _>>()?;
            members.sort_by_key(|m| {
                (
                    m.season.unwrap_or(i32::MAX),
                    m.episode.unwrap_or(i32::MAX),
                    m.title.clone(),
                )
            });
            return Ok(members);
        }
        let selected_tmdb = name
            .strip_prefix("tmdb:tv:")
            .and_then(|raw| raw.parse::<i64>().ok());
        if name.starts_with("tmdb:tv:") && selected_tmdb.is_none() {
            return Err("Identifiant de série invalide.".into());
        }
        let mut members: Vec<_> = self
            .catalog(profile_id)?
            .into_iter()
            .filter(|m| {
                m.category == "serie"
                    && match selected_tmdb {
                        Some(id) => m.tmdb_id == Some(id),
                        // Legacy selectors are display labels.  Treat harmless
                        // casing/accent/spacing variants as one label, then
                        // retain the existing ambiguity check below so a real
                        // homonym still cannot select the wrong show.
                        None => {
                            chill_catalog::database::series_title_key(
                                m.series.as_deref().unwrap_or(&m.title),
                            ) == chill_catalog::database::series_title_key(name)
                        }
                    }
            })
            .collect();
        if selected_tmdb.is_none() {
            let groups = members
                .iter()
                .filter_map(|media| {
                    media
                        .tmdb_id
                        .map(|id| format!("tmdb:tv:{id}"))
                        .or_else(|| chill_catalog::database::series_source_key(media))
                })
                .collect::<std::collections::HashSet<_>>();
            if groups.len() > 1 {
                return Err("Plusieurs séries portent ce nom. Ouvrez-la depuis sa vignette pour choisir la bonne fiche.".into());
            }
        }
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

    /// Compact profile-only read, independent of catalogue delta revisions.
    /// Legacy resumes without a measured duration are intentionally omitted.
    pub fn playback_progress(
        &self,
        profile: &str,
        known: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        self.require_profile(profile)?;
        let epoch: i64 = self
            .personal
            .query_row(
                "SELECT revision FROM cc_personal_epoch WHERE profile_id=?1",
                [profile],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        // Include a format marker: an older schema's token cannot hide new data.
        let revision = format!("measured-1:{profile}:{epoch}");
        if known == Some(revision.as_str()) {
            return Ok(json!({"revision":revision,"unchanged":true}));
        }
        let mut query = self.personal.prepare(
            "SELECT s.stable_media_id,s.position_ms,d.duration_ms,s.watched FROM cc_profile_state s
             JOIN cc_resume_measurement d USING(profile_id,stable_media_id)
             WHERE s.profile_id=?1 AND s.position_ms>0 ORDER BY s.stable_media_id LIMIT 30001"
        ).map_err(db_error)?;
        let items = query
            .query_map([profile], |row| {
                Ok(json!({
                    "media_id":row.get::<_,String>(0)?,"position_ms":row.get::<_,i64>(1)?,
                    "duration_ms":row.get::<_,i64>(2)?,"watched":row.get::<_,bool>(3)?
                }))
            })
            .map_err(db_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db_error)?;
        if items.len() > 30000 {
            return Err("PROGRESS_TOO_LARGE".into());
        }
        Ok(json!({"revision":revision,"unchanged":false,"items":items}))
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
            ("series", Some(name)) if item.category == "serie" => {
                if self
                    .series(profile, name)?
                    .iter()
                    .any(|member| member.id == media)
                {
                    Ok(())
                } else {
                    Err("Cette vidéo n’appartient pas à cette série.".into())
                }
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
        let mut order_unknown = false;
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
                let members = self.series(profile, &format!("media:{}", current.id))?;
                if current.category != "serie"
                    || current.season.is_none()
                    || current.episode.is_none()
                {
                    if let Some(ordered) = flat_series_order(&members) {
                        ordered
                            .iter()
                            .position(|m| m.id == media)
                            .and_then(|index| ordered.get(index + 1))
                            .map(|m| m.id.clone())
                    } else {
                        order_unknown = true;
                        None
                    }
                } else {
                    let next = members
                        .iter()
                        .find(|m| (m.season, m.episode) > (current.season, current.episode))
                        .map(|m| m.id.clone());
                    order_unknown = next.is_none()
                        && members
                            .iter()
                            .any(|m| m.season.is_none() || m.episode.is_none());
                    next
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
            order_unknown,
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
        let revision = self.media_revision(stable_id)?;
        self.update_media_at_revision(profile_id, stable_id, revision, patch)
            .map(|(media, _)| media)
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
        let updated = crate::metadata_edit::apply_common_patch(&tx, &legacy, patch)?;
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
            self.series(profile_id, &format!("media:{}", target.id))?
                .into_iter()
                .filter(|m| scope == "series" || m.season == target.season)
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
        self.save_validated_progress_with_short_tail(profile_id, stable_id, progress, false)
    }

    /// `completed_short_tail` is an internal server proof, never a client field:
    /// the active lease observed the complete final tenth without a seek/gap.
    pub fn save_validated_progress_with_short_tail(
        &self,
        profile_id: &str,
        stable_id: &str,
        progress: &Progress,
        completed_short_tail: bool,
    ) -> Result<PersonalState, String> {
        self.require_profile(profile_id)?;
        if progress.media_id != stable_id
            || progress.duration_ms == 0
            || progress.duration_ms > i64::MAX as u64
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
        let short_video = progress.duration_ms < 100_000;
        let watched = old.watched
            || if short_video {
                completed_short_tail
                    && progress.position_ms == progress.duration_ms
                    && !progress.seek
                    && !progress.buffering
            } else {
                final_watched_ms >= WATCHED_THRESHOLD_MS
            };
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
        tx.execute("INSERT INTO cc_resume_measurement(profile_id,stable_media_id,duration_ms) VALUES(?1,?2,?3)
            ON CONFLICT(profile_id,stable_media_id) DO UPDATE SET duration_ms=excluded.duration_ms",
            params![profile_id,stable_id,progress.duration_ms as i64]).map_err(db_error)?;
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
            if collection.kind == "saga" {
                collection.media_ids = self.saga_order(&collection.id)?;
                collection.poster = collection
                    .poster
                    .as_ref()
                    .map(|_| format!("/api/2026-10-14/sagas/{}/artwork", collection.id));
            }
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
                "SELECT p.id,p.name,p.revision,a.poster FROM cc_playlists p LEFT JOIN cc_playlist_artwork a ON a.playlist_id=p.id WHERE p.profile_id=?1 ORDER BY p.name,p.id",
            )
            .map_err(db_error)?;
        stmt.query_map([profile_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, Option<String>>(3)?,
            ))
        })
        .map_err(db_error)?
        .map(|row| {
            let (id, name, revision, poster) = row.map_err(db_error)?;
            let artwork = poster
                .map(|_| format!("/api/2026-10-14/profiles/{profile_id}/playlists/{id}/artwork"));
            Ok(PlaylistView {
                media_ids: self.playlist_members(&id)?,
                id,
                name,
                revision: revision as u64,
                poster: artwork,
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
                let order = self.saga_order(&c.id)?;
                let media_ids = order
                    .iter()
                    .map(|legacy| self.stable_id(legacy))
                    .collect::<Result<Vec<_>, _>>()?;
                let poster = c
                    .poster
                    .as_ref()
                    .map(|_| format!("/api/2026-10-14/sagas/{}/artwork", c.id));
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
        self.reorder_common_saga(id, revision, expected, desired)?;
        self.saga(id)
    }
    pub fn delete_saga_at_revision(&mut self, id: &str, revision: u64) -> Result<(), String> {
        self.catalogue.delete_saga_at_revision(id, revision)
    }

    pub fn playlist(&self, profile_id: &str, id: &str) -> Result<PlaylistView, String> {
        self.require_playlist(profile_id, id)?;
        let (name, revision, poster): (String, i64, Option<String>) = self
            .personal
            .query_row(
                "SELECT p.name,p.revision,a.poster FROM cc_playlists p LEFT JOIN cc_playlist_artwork a ON a.playlist_id=p.id WHERE p.id=?1 AND p.profile_id=?2",
                params![id, profile_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map_err(db_error)?;
        Ok(PlaylistView {
            id: id.into(),
            name,
            revision: revision as u64,
            media_ids: self.playlist_members(id)?,
            poster: poster
                .map(|_| format!("/api/2026-10-14/profiles/{profile_id}/playlists/{id}/artwork")),
        })
    }

    pub fn set_playlist_artwork_at_revision(
        &self,
        profile: &str,
        playlist: &str,
        expected: u64,
        bytes: &[u8],
        cache: &Path,
    ) -> Result<(), String> {
        self.require_playlist(profile, playlist)?;
        if bytes.len() > 8 * 1024 * 1024 {
            return Err("Image trop volumineuse.".into());
        }
        let tx = self.personal.unchecked_transaction().map_err(db_error)?;
        let revision: i64 = tx
            .query_row(
                "SELECT revision FROM cc_playlists WHERE id=?1 AND profile_id=?2",
                params![playlist, profile],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        if revision as u64 != expected {
            return Err("La playlist a changé entre-temps.".into());
        }
        let old: Option<String> = tx
            .query_row(
                "SELECT poster FROM cc_playlist_artwork WHERE playlist_id=?1",
                [playlist],
                |r| r.get(0),
            )
            .optional()
            .map_err(db_error)?;
        let poster = chill_catalog::poster::store(bytes, cache)?;
        if tx.execute("UPDATE cc_playlists SET revision=revision+1 WHERE id=?1 AND profile_id=?2 AND revision=?3", params![playlist,profile,revision]).map_err(db_error)? != 1 { return Err("La playlist a changé entre-temps.".into()); }
        tx.execute("INSERT INTO cc_playlist_artwork(playlist_id,poster) VALUES(?1,?2) ON CONFLICT(playlist_id) DO UPDATE SET poster=excluded.poster", params![playlist,poster]).map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        if let Some(old) = old {
            let _ = self.remove_unreferenced_playlist_image(&old, cache);
        }
        Ok(())
    }
    fn remove_unreferenced_playlist_image(&self, old: &str, cache: &Path) -> Result<(), String> {
        let path = Path::new(old);
        if path.parent() != Some(cache)
            || !path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("custom-"))
        {
            return Ok(());
        }
        let used:bool=self.personal.query_row("SELECT EXISTS(SELECT 1 FROM cc_playlist_artwork WHERE poster=?1 UNION ALL SELECT 1 FROM collection_artwork WHERE poster=?1 UNION ALL SELECT 1 FROM media WHERE json_extract(data,'$.poster')=?1)",[old],|r|r.get(0)).map_err(db_error)?;
        if !used
            && let Ok(m) = std::fs::symlink_metadata(path)
            && m.file_type().is_file()
        {
            let _ = std::fs::remove_file(path);
        }
        Ok(())
    }

    pub fn playlist_artwork_path(&self, profile: &str, playlist: &str) -> Result<PathBuf, String> {
        self.require_playlist(profile, playlist)?;
        self.personal
            .query_row(
                "SELECT poster FROM cc_playlist_artwork WHERE playlist_id=?1",
                [playlist],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .map_err(db_error)?
            .map(PathBuf::from)
            .ok_or_else(|| "Affiche introuvable.".into())
    }

    /// Captures profile-facing catalogue state while the caller owns the Store
    /// actor. The caller may safely page this immutable value after the call.
    pub fn sync_snapshot(&mut self, profile: &str) -> Result<SyncSnapshot, String> {
        self.sync_snapshot_with_probe(profile, || {})
    }
    fn sync_snapshot_with_probe(
        &mut self,
        profile: &str,
        after_media: impl FnOnce(),
    ) -> Result<SyncSnapshot, String> {
        self.require_profile(profile)?;
        let before = self.sync_revision(profile)?;
        let (count, bytes): (i64, i64) = self
            .personal
            .query_row(
                "SELECT COUNT(*),COALESCE(SUM(length(CAST(data AS BLOB))),0) FROM media",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(db_error)?;
        if count > 10_000 {
            return Err("SYNC_CATALOG_TOO_LARGE".into());
        }
        if bytes > 32 * 1024 * 1024 {
            return Err("SYNC_SNAPSHOT_TOO_LARGE".into());
        }
        let missing:bool=self.personal.query_row("SELECT EXISTS(SELECT 1 FROM media m LEFT JOIN cc_media_identity i ON i.legacy_id=m.id WHERE i.stable_id IS NULL)",[],|r|r.get(0)).map_err(db_error)?;
        if missing {
            self.ensure_identities(&self.catalogue.list()?)?;
        }
        // One joined query instead of several point queries per media on the Pi.
        let mut statement=self.personal.prepare("SELECT m.data,i.stable_id,i.relative_path,COALESCE(r.revision,1),COALESCE(s.watched,0),s.rating FROM media m JOIN cc_media_identity i ON i.legacy_id=m.id LEFT JOIN cc_media_revision r ON r.stable_id=i.stable_id LEFT JOIN cc_profile_state s ON s.stable_media_id=i.stable_id AND s.profile_id=?1 ORDER BY m.id").map_err(db_error)?;
        let rows = statement
            .query_map([profile], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, bool>(4)?,
                    r.get::<_, Option<u8>>(5)?,
                ))
            })
            .map_err(db_error)?;
        let items = rows
            .map(|r| {
                let (raw, id, path, revision, watched, rating) = r.map_err(db_error)?;
                let mut media: Media = serde_json::from_str(&raw).map_err(|_| "SYNC_INVALID")?;
                media.id = id;
                media.path = path;
                media.watched = watched;
                media.rating = rating;
                if media.poster.is_some() {
                    media.poster = Some(format!(
                        "/api/2026-10-14/profiles/{profile}/media/{}/artwork",
                        media.id
                    ));
                }
                Ok(SyncMedia {
                    media,
                    revision: revision as u64,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        drop(statement);
        after_media();
        let captured_revision = self.sync_revision(profile)?;
        let series_revision = captured_revision.0;
        let mut series_revisions = std::collections::BTreeMap::new();
        let mut legacy_series_groups =
            std::collections::BTreeMap::<String, std::collections::BTreeSet<String>>::new();
        for item in &items {
            if item.media.category == "serie" {
                let key = item
                    .media
                    .tmdb_id
                    .map(|id| format!("tmdb:tv:{id}"))
                    .unwrap_or_else(|| {
                        item.media
                            .series
                            .clone()
                            .unwrap_or_else(|| item.media.title.clone())
                    });
                series_revisions.insert(key.clone(), series_revision);
                series_revisions.insert(format!("media:{}", item.media.id), series_revision);
                let name = item
                    .media
                    .series
                    .clone()
                    .unwrap_or_else(|| item.media.title.clone());
                let identity = item
                    .media
                    .tmdb_id
                    .map(|id| format!("tmdb:tv:{id}"))
                    .or_else(|| chill_catalog::database::series_source_key(&item.media))
                    .unwrap();
                legacy_series_groups
                    .entry(name)
                    .or_default()
                    .insert(identity);
            }
        }
        for (name, groups) in legacy_series_groups {
            if groups.len() == 1 {
                series_revisions.insert(name, series_revision);
            }
        }
        let known_seasons = items
            .iter()
            .filter_map(|item| {
                Some((
                    item.media.tmdb_id?,
                    item.media.season?,
                    item.media.category == "serie",
                ))
            })
            .filter(|(_, _, series)| *series)
            .map(|(tv_id, season, _)| (tv_id, season))
            .collect::<std::collections::BTreeSet<_>>();
        let mut season_metadata = std::collections::BTreeMap::new();
        for (tv_id, season) in known_seasons {
            if let Some(mut metadata) = self.season_provider(tv_id, season)? {
                // The record stores a server-local cached path.  Sync DTOs,
                // including durable delta checkpoints, must never contain it.
                if metadata.poster.is_some() {
                    metadata.poster = Some(format!(
                        "/api/2026-10-14/profiles/{profile}/series/{tv_id}/seasons/{season}/artwork"
                    ));
                }
                season_metadata.insert(format!("tmdb:tv:{tv_id}:season:{season}"), metadata);
            }
        }
        let mut snapshot = SyncSnapshot {
            items,
            sagas: self.sagas()?,
            playlists: self.playlists(profile)?,
            series_revisions,
            season_metadata,
            serialized_bytes: 0,
            captured_revision,
        };
        let after = self.sync_revision(profile)?;
        if before != after {
            return Err("SYNC_SNAPSHOT_CHANGED".into());
        }
        snapshot.captured_revision = after;
        struct Budget(usize);
        impl std::io::Write for Budget {
            fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
                self.0 = self.0.saturating_add(data.len());
                if self.0 > 32 * 1024 * 1024 {
                    return Err(std::io::Error::other("snapshot limit"));
                }
                Ok(data.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut budget = Budget(0);
        serde_json::to_writer(&mut budget, &snapshot)
            .map_err(|_| "SYNC_SNAPSHOT_TOO_LARGE".to_owned())?;
        snapshot.serialized_bytes = budget.0;
        Ok(snapshot)
    }
    pub fn sync_revision(&self, profile: &str) -> Result<(u64, u64), String> {
        self.require_profile(profile)?;
        let personal: i64 = self
            .personal
            .query_row(
                "SELECT revision FROM cc_sync_personal_epoch WHERE profile_id=?1",
                [profile],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        Ok((self.common_revision()?, personal as u64))
    }

    /// This is intentionally opt-in and only called by the desktop client's
    /// private SQLite cache. Server startup never invokes it.
    pub fn replace_client_mirror(
        &mut self,
        profile: &str,
        items: &[SyncMedia],
        cache_document: &Value,
    ) -> Result<(), String> {
        if items.len() > 10_000 {
            return Err("SYNC_CATALOG_TOO_LARGE".into());
        }
        let encoded =
            serde_json::to_string(cache_document).map_err(|_| "SYNC_INVALID".to_string())?;
        if encoded.len() > 32 * 1024 * 1024 {
            return Err("SYNC_SNAPSHOT_TOO_LARGE".into());
        }
        let mut ids = HashSet::new();
        for item in items {
            if Uuid::parse_str(&item.media.id).is_err() || !ids.insert(item.media.id.as_str()) {
                return Err("SYNC_INVALID".into());
            }
        }
        self.personal.execute_batch("CREATE TABLE IF NOT EXISTS cc_client_mirror (id INTEGER PRIMARY KEY CHECK(id=1), profile_id TEXT NOT NULL, document TEXT NOT NULL);").map_err(db_error)?;
        let tx = self.personal.unchecked_transaction().map_err(db_error)?;
        // A client cache is scoped to exactly one (server, profile) database.
        // Use the stable id as the local legacy id too: it is opaque and no path
        // is opened by Store browsing operations.
        tx.execute("DELETE FROM cc_playlist_artwork", [])
            .map_err(db_error)?;
        tx.execute("DELETE FROM cc_playlist_members", [])
            .map_err(db_error)?;
        tx.execute("DELETE FROM cc_playlists", [])
            .map_err(db_error)?;
        tx.execute_batch("DELETE FROM collection_positions;DELETE FROM collection_members;DELETE FROM collection_artwork;DELETE FROM collection_revisions;DELETE FROM imported_sagas;DELETE FROM collections;").map_err(db_error)?;
        tx.execute("DELETE FROM cc_profile_state", [])
            .map_err(db_error)?;
        tx.execute("DELETE FROM cc_media_revision", [])
            .map_err(db_error)?;
        tx.execute("DELETE FROM cc_media_identity", [])
            .map_err(db_error)?;
        tx.execute("DELETE FROM media", []).map_err(db_error)?;
        tx.execute(
            "INSERT OR IGNORE INTO cc_profiles(id,name) VALUES(?1,?1)",
            [profile],
        )
        .map_err(db_error)?;
        for item in items {
            let stable = &item.media.id;
            let serialized =
                serde_json::to_string(&item.media).map_err(|_| "SYNC_INVALID".to_string())?;
            tx.execute(
                "INSERT INTO media(id,data) VALUES(?1,?2)",
                params![stable, serialized],
            )
            .map_err(db_error)?;
            tx.execute(
                "INSERT INTO cc_media_identity(stable_id,legacy_id,relative_path) VALUES(?1,?1,?2)",
                params![stable, item.media.path],
            )
            .map_err(db_error)?;
            tx.execute(
                "INSERT INTO cc_media_revision(stable_id,revision) VALUES(?1,?2)",
                params![stable, item.revision as i64],
            )
            .map_err(db_error)?;
            tx.execute("INSERT INTO cc_profile_state(profile_id,stable_media_id,watched,rating) VALUES(?1,?2,?3,?4)", params![profile,stable,item.media.watched,item.media.rating]).map_err(db_error)?;
        }
        let sagas: Vec<SagaView> =
            serde_json::from_value(cache_document["sagas"].clone()).map_err(|_| "SYNC_INVALID")?;
        for saga in sagas {
            let id = saga.id.parse::<i64>().map_err(|_| "SYNC_INVALID")?;
            if id <= 0 || saga.revision > i64::MAX as u64 {
                return Err("SYNC_INVALID".into());
            }
            tx.execute(
                "INSERT INTO collections(id,name,kind) VALUES(?1,?2,'saga')",
                params![id, saga.name],
            )
            .map_err(db_error)?;
            tx.execute(
                "INSERT INTO collection_revisions(collection_id,revision) VALUES(?1,?2)",
                params![id, saga.revision as i64],
            )
            .map_err(db_error)?;
            for (position, media) in saga.media_ids.iter().enumerate() {
                if !ids.contains(media.as_str()) {
                    return Err("SYNC_INVALID".into());
                }
                tx.execute(
                    "INSERT INTO collection_members(collection_id,media_id) VALUES(?1,?2)",
                    params![id, media],
                )
                .map_err(db_error)?;
                tx.execute("INSERT INTO collection_positions(collection_id,media_id,position) VALUES(?1,?2,?3)",params![id,media,position as i64]).map_err(db_error)?;
            }
        }
        tx.execute("DELETE FROM cc_client_mirror", [])
            .map_err(db_error)?;
        tx.execute(
            "INSERT INTO cc_client_mirror(id,profile_id,document) VALUES(1,?1,?2)",
            params![profile, encoded],
        )
        .map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        Ok(())
    }
    pub fn client_mirror_document(&self, profile: &str) -> Result<Option<Value>, String> {
        let exists: bool = self.personal.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='cc_client_mirror')",
            [], |r| r.get(0),
        ).map_err(db_error)?;
        if !exists {
            return Ok(None);
        }
        let value: Option<String> = self
            .personal
            .query_row(
                "SELECT document FROM cc_client_mirror WHERE id=1 AND profile_id=?1",
                [profile],
                |r| r.get(0),
            )
            .optional()
            .map_err(db_error)?;
        value
            .map(|v| serde_json::from_str(&v).map_err(|_| "Cache local invalide.".into()))
            .transpose()
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
    pub fn delete_playlist_with_artwork(
        &self,
        profile: &str,
        id: &str,
        cache: &Path,
    ) -> Result<(), String> {
        self.require_playlist(profile, id)?;
        let old = self.playlist_artwork_path(profile, id).ok();
        self.delete_collection(profile, id)?;
        if let Some(path) = old {
            let _ = self.remove_unreferenced_playlist_image(&path.to_string_lossy(), cache);
        }
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
        let tx =
            rusqlite::Transaction::new_unchecked(&self.personal, TransactionBehavior::Deferred)
                .map_err(db_error)?;
        let profiles = self.profiles()?;
        let mut posters = Vec::new();
        let has_playlist_artwork: bool = self
            .personal
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM cc_playlist_artwork)",
                [],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        out.write_all(if has_playlist_artwork {
            br#"{"format_version":2,"export_id":"#
        } else {
            br#"{"format_version":1,"export_id":"#
        })
        .map_err(|_| "Export de staging inaccessible.")?;
        serde_json::to_writer(&mut *out, export_id).map_err(|_| "Export invalide.")?;
        out.write_all(b",\"profiles\":")
            .map_err(|_| "Export de staging inaccessible.")?;
        write_json_array(out, profiles.iter(), |out, profile| {
            serde_json::to_writer(out, profile).map_err(|_| "Export invalide.".into())
        })?;
        out.write_all(b",\"media\":[")
            .map_err(|_| "Export de staging inaccessible.")?;
        let mut statement = self
            .personal
            .prepare("SELECT data FROM media ORDER BY id")
            .map_err(db_error)?;
        let rows = statement
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(db_error)?;
        for (index, row) in rows.enumerate() {
            let item: Media = serde_json::from_str(&row.map_err(db_error)?)
                .map_err(|_| "Catalogue local endommagé.")?;
            if index != 0 {
                out.write_all(b",")
                    .map_err(|_| "Export de staging inaccessible.")?;
            }
            let stable = self.stable_id(&item.id)?;
            let mut data = scrub_media(&item);
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
        let mut sagas = saga_export(&self.personal, self)?;
        for saga in &mut sagas {
            let id = saga.id.parse::<i64>().map_err(|_| "Saga invalide.")?;
            let path: Option<String> = self
                .personal
                .query_row(
                    "SELECT poster FROM collection_artwork WHERE collection_id=?1",
                    [id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(db_error)?;
            if let Some(relative) = path
                .as_ref()
                .and_then(|p| Path::new(p).strip_prefix(poster_root).ok())
                .filter(|p| {
                    p.components()
                        .all(|c| matches!(c, std::path::Component::Normal(_)))
                })
            {
                let reference = format!("posters/saga-{id}.bin");
                saga.poster = Some(reference.clone());
                posters.push(ArchivePoster {
                    reference,
                    relative: relative.to_path_buf(),
                });
            }
        }
        serde_json::to_writer(&mut *out, &sagas).map_err(|_| "Export invalide.")?;
        out.write_all(b",\"personal\":[")
            .map_err(|_| "Export de staging inaccessible.")?;
        for (index, profile) in profiles.iter().enumerate() {
            if index != 0 {
                out.write_all(b",")
                    .map_err(|_| "Export de staging inaccessible.")?;
            }
            let mut personal = self.export_profile(profile)?;
            if let Some(playlists) = personal.get_mut("playlists").and_then(Value::as_array_mut) {
                for playlist in playlists {
                    let Some(id) = playlist.get("id").and_then(Value::as_str) else {
                        continue;
                    };
                    let path: Option<String> = self
                        .personal
                        .query_row(
                            "SELECT poster FROM cc_playlist_artwork WHERE playlist_id=?1",
                            [id],
                            |r| r.get(0),
                        )
                        .optional()
                        .map_err(db_error)?;
                    if let Some(relative) = path
                        .as_deref()
                        .and_then(|p| Path::new(p).strip_prefix(poster_root).ok())
                        .filter(|p| {
                            p.components()
                                .all(|c| matches!(c, std::path::Component::Normal(_)))
                        })
                    {
                        let reference = format!("posters/playlist-{id}.bin");
                        if let Some(object) = playlist.as_object_mut() {
                            object.insert("poster_ref".into(), Value::String(reference.clone()));
                        }
                        posters.push(ArchivePoster {
                            reference,
                            relative: relative.to_path_buf(),
                        });
                    }
                }
            }
            serde_json::to_writer(&mut *out, &personal).map_err(|_| "Export invalide.")?;
        }
        out.write_all(b"],\"leases\":[]}")
            .map_err(|_| "Export de staging inaccessible.")?;
        drop(statement);
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
    pub(crate) fn require_profile(&self, id: &str) -> Result<(), String> {
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
    pub(crate) fn refresh_series_groups(&self) -> Result<(), String> {
        let revision = i64::try_from(self.common_revision()?).map_err(|_| "Révision invalide.")?;
        // This is a derived, disposable index.  A dedicated rule marker makes
        // the first read after an algorithm upgrade rebuild it once, while an
        // unchanged catalogue never pays the rebuild cost on every page.
        self.personal.execute_batch("CREATE TABLE IF NOT EXISTS cc_series_group_cache(id INTEGER PRIMARY KEY CHECK(id=1),revision INTEGER NOT NULL,rule_version INTEGER NOT NULL);").map_err(db_error)?;
        let cached: Option<(i64, i64)> = self
            .personal
            .query_row(
                "SELECT revision,rule_version FROM cc_series_group_cache WHERE id=1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(db_error)?;
        if cached == Some((revision, chill_catalog::database::SERIES_GROUP_RULE_VERSION)) {
            return Ok(());
        }
        let media = self.catalogue.list()?;
        self.ensure_identities(&media)?;
        let tx = rusqlite::Transaction::new_unchecked(
            &self.personal,
            rusqlite::TransactionBehavior::Immediate,
        )
        .map_err(db_error)?;
        if i64::try_from(self.common_revision()?).map_err(|_| "Révision invalide.")? != revision {
            return Err(
                "Le catalogue a changé pendant la préparation des séries. Réessayez.".into(),
            );
        }
        tx.execute("DELETE FROM cc_series_group", [])
            .map_err(db_error)?;
        for item in media {
            if let Some(key) = chill_catalog::database::series_source_key(&item) {
                tx.execute(
                    "INSERT INTO cc_series_group(legacy_id,source_key) VALUES(?1,?2)",
                    params![item.id, key],
                )
                .map_err(db_error)?;
            }
        }
        tx.execute(
            "INSERT INTO cc_series_group_cache(id,revision,rule_version) VALUES(1,?1,?2) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision,rule_version=excluded.rule_version",
            params![revision, chill_catalog::database::SERIES_GROUP_RULE_VERSION],
        ).map_err(db_error)?;
        tx.commit().map_err(db_error)
    }
    pub(crate) fn stable_id(&self, legacy: &str) -> Result<String, String> {
        self.personal
            .query_row(
                "SELECT stable_id FROM cc_media_identity WHERE legacy_id=?1",
                [legacy],
                |r| r.get(0),
            )
            .map_err(media_lookup_error)
    }
    pub(crate) fn legacy_id(&self, stable: &str) -> Result<String, String> {
        self.personal
            .query_row(
                "SELECT legacy_id FROM cc_media_identity WHERE stable_id=?1",
                [stable],
                |r| r.get(0),
            )
            .map_err(media_lookup_error)
    }
    pub(crate) fn profile_view(&self, profile: &str, mut media: Media) -> Result<Media, String> {
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
                "/api/2026-10-14/profiles/{profile}/media/{}/artwork",
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
    // Publish CAS revisions in the same transaction, not lazily through the
    // unbounded legacy catalogue reader. Existing revisions are never reset.
    tx.execute("INSERT OR IGNORE INTO cc_media_revision(stable_id,revision) SELECT stable_id,1 FROM cc_media_identity",[]).map_err(db_error)?;
    Ok(())
}
pub(crate) fn db_error(_: rusqlite::Error) -> String {
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
                "SELECT m.media_id FROM collection_members m LEFT JOIN collection_positions p ON p.collection_id=m.collection_id AND p.media_id=m.media_id JOIN media d ON d.id=m.media_id WHERE m.collection_id=?1 ORDER BY p.position IS NULL,p.position,json_extract(d.data,'$.year') IS NULL,json_extract(d.data,'$.year'),json_extract(d.data,'$.title') COLLATE NOCASE,m.rowid",
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
    fn snapshot_rejects_a_scanner_commit_between_rows_and_revision() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("race.sqlite");
        let mut reader = Store::open(&path).unwrap();
        let profile = reader.create_profile("Reader").unwrap();
        reader
            .snapshot(vec![media("A.mp4", "/fixture/A.mp4")])
            .unwrap();
        let old = reader.sync_snapshot(&profile.id).unwrap();
        let mut scanner = Store::open(&path).unwrap();
        let result = reader.sync_snapshot_with_probe(&profile.id, || {
            scanner
                .snapshot(vec![
                    media("A.mp4", "/fixture/A.mp4"),
                    media("B.mp4", "/fixture/B.mp4"),
                ])
                .unwrap();
        });
        assert_eq!(result.unwrap_err(), "SYNC_SNAPSHOT_CHANGED");
        let retry = reader.sync_snapshot(&profile.id).unwrap();
        assert_ne!(retry.captured_revision, old.captured_revision);
        assert_eq!(old.items.len(), 1);
        assert_eq!(retry.items.len(), 2);
        assert!(retry.items.iter().any(|item| item.media.title == "B.mp4"));
        assert_eq!(
            retry.captured_revision,
            reader.sync_revision(&profile.id).unwrap()
        );
    }
    #[test]
    fn sync_snapshot_carries_season_metadata_without_server_path() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("season.sqlite")).unwrap();
        let profile = store.create_profile("Viewer").unwrap();
        let mut episode = media("Doctor/S01/E01.mkv", "/fixture/Doctor/Season 01/E01.mkv");
        episode.category = "serie".into();
        episode.series = Some("Doctor Who".into());
        episode.season = Some(1);
        episode.episode = Some(1);
        episode.tmdb_id = Some(57243);
        store.snapshot(vec![episode]).unwrap();
        let provider = SeasonProvider {
            tv_id: 57243,
            season: 1,
            name: "Season 1".into(),
            overview: "Shared season synopsis".into(),
            poster: Some("/private/posters/season.jpg".into()),
        };
        store
            .personal
            .execute(
                "INSERT INTO cc_season_provider(tv_id,season,data,updated_at) VALUES(?1,?2,?3,1)",
                params![57243, 1, serde_json::to_string(&provider).unwrap()],
            )
            .unwrap();
        let snapshot = store.sync_snapshot(&profile.id).unwrap();
        let season = snapshot
            .season_metadata
            .get("tmdb:tv:57243:season:1")
            .unwrap();
        assert_eq!(season.overview, "Shared season synopsis");
        let expected = format!(
            "/api/2026-10-14/profiles/{}/series/57243/seasons/1/artwork",
            profile.id
        );
        assert_eq!(season.poster.as_deref(), Some(expected.as_str()));
    }
    #[test]
    fn scan_publishes_new_media_revision_without_legacy_catalog_warmup() {
        let dir = tempdir().unwrap();
        let mut s = Store::open(&dir.path().join("catalog.sqlite")).unwrap();
        s.snapshot(vec![media("New.mp4", "/nas/New.mp4")]).unwrap();
        let id = s.stable_id("New.mp4").unwrap();
        assert_eq!(s.media_revision(&id).unwrap(), 1);
        assert_eq!(s.fingerprint_target(&id).unwrap().media_revision, 1);
        assert_eq!(s.fingerprint_status(&id).unwrap().state, "not_requested");
        s.mark_fingerprint_queued(&id, crate::fingerprint::FingerprintKind::Partial)
            .unwrap();
        let queued = s.fingerprint_status(&id).unwrap();
        assert_eq!(queued.state, "queued");
        assert_eq!(queued.strength, None);
        s.mark_fingerprint_running(&id, 1).unwrap();
        assert_eq!(s.fingerprint_status(&id).unwrap().state, "running");
    }
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
    fn group_ratings_without_series_names_use_effective_title_and_stay_personal() {
        let dir = tempdir().unwrap();
        let mut s = Store::open(&dir.path().join("ratings.sqlite")).unwrap();
        let mut items = Vec::new();
        for (path, title, season) in [
            ("a1.mkv", "Anonymous A", 1),
            ("a2.mkv", "Anonymous A", 2),
            ("b1.mkv", "Anonymous B", 1),
        ] {
            let mut m = media(path, &format!("/nas/{path}"));
            m.category = "serie".into();
            m.series = None;
            m.title = title.into();
            m.season = Some(season);
            items.push(m);
        }
        s.snapshot(items).unwrap();
        let a = s.create_profile("A").unwrap();
        let other = s.create_profile("Other").unwrap();
        let items = s.catalog(&a.id).unwrap();
        let a1 = items
            .iter()
            .find(|m| m.title == "Anonymous A" && m.season == Some(1))
            .unwrap()
            .id
            .clone();
        let a2 = items
            .iter()
            .find(|m| m.title == "Anonymous A" && m.season == Some(2))
            .unwrap()
            .id
            .clone();
        let b1 = items
            .iter()
            .find(|m| m.title == "Anonymous B")
            .unwrap()
            .id
            .clone();
        s.set_rating(&a.id, &a1, Some(4), "series").unwrap();
        assert_eq!(s.media(&a.id, &a1).unwrap().rating, Some(4));
        assert_eq!(s.media(&a.id, &a2).unwrap().rating, Some(4));
        assert_eq!(s.media(&a.id, &b1).unwrap().rating, None);
        s.set_rating(&a.id, &a1, None, "season").unwrap();
        assert_eq!(s.media(&a.id, &a1).unwrap().rating, None);
        assert_eq!(s.media(&a.id, &a2).unwrap().rating, Some(4));
        s.set_rating(&a.id, &b1, Some(2), "season").unwrap();
        assert_eq!(s.media(&a.id, &a1).unwrap().rating, None);
        assert_eq!(s.media(&a.id, &b1).unwrap().rating, Some(2));
        for id in [&a1, &a2, &b1] {
            assert_eq!(s.media(&other.id, id).unwrap().rating, None);
        }
    }
    #[test]
    fn next_saga_rechecks_reordered_members_and_never_skips_an_unavailable_next() {
        let dir = tempdir().unwrap();
        let mut s = Store::open(&dir.path().join("next.sqlite")).unwrap();
        let a = media("a.mp4", "/nas/a.mp4");
        let b = media("b.mp4", "/nas/b.mp4");
        let c = media("c.mp4", "/nas/c.mp4");
        s.snapshot(vec![a.clone(), b.clone(), c]).unwrap();
        let p = s.create_profile("Next").unwrap();
        let items = s.catalog(&p.id).unwrap();
        let id = |path: &str| items.iter().find(|m| m.path == path).unwrap().id.clone();
        let ids = vec![id("a.mp4"), id("b.mp4"), id("c.mp4")];
        assert!(
            s.next_video(
                &p.id,
                &ids[0],
                &PlaybackContext {
                    kind: "standalone".into(),
                    id: None
                }
            )
            .unwrap()
            .media
            .is_none()
        );
        let saga = s.create_saga("Next saga").unwrap();
        let saga = s
            .saga_members_at_revision(&saga.id, &ids, saga.revision, true)
            .unwrap();
        let context = PlaybackContext {
            kind: "saga".into(),
            id: Some(saga.id.clone()),
        };
        assert_eq!(
            s.next_video(&p.id, &ids[0], &context)
                .unwrap()
                .media
                .unwrap()
                .id,
            ids[1]
        );
        s.reorder_saga_at_revision(
            &saga.id,
            saga.revision,
            &ids,
            &[ids[0].clone(), ids[2].clone(), ids[1].clone()],
        )
        .unwrap();
        assert_eq!(
            s.next_video(&p.id, &ids[0], &context)
                .unwrap()
                .media
                .unwrap()
                .id,
            ids[2]
        );
        s.snapshot(vec![a, b]).unwrap();
        let missing = s.next_video(&p.id, &ids[0], &context).unwrap();
        assert_eq!(missing.media.unwrap().id, ids[2]);
        assert!(!missing.available);
        assert!(!missing.new_season);
        assert!(
            s.next_video(&p.id, &ids[1], &context)
                .unwrap()
                .media
                .is_none()
        );
    }
    #[test]
    fn next_series_stops_for_unknown_numbers_and_marks_a_known_season_transition() {
        let dir = tempdir().unwrap();
        let mut s = Store::open(&dir.path().join("series-next.sqlite")).unwrap();
        let mut items = Vec::new();
        for (path, season, episode) in [
            ("known.mp4", Some(1), Some(1)),
            ("new.mp4", Some(2), Some(1)),
            ("unknown-episode.mp4", Some(1), None),
            ("unknown-season.mp4", None, Some(1)),
        ] {
            let mut m = media(path, &format!("/nas/{path}"));
            m.category = "serie".into();
            m.series = Some("Next".into());
            m.season = season;
            m.episode = episode;
            items.push(m);
        }
        s.snapshot(items).unwrap();
        let p = s.create_profile("Next").unwrap();
        let items = s.catalog(&p.id).unwrap();
        let id = |path: &str| items.iter().find(|m| m.path == path).unwrap().id.clone();
        let context = PlaybackContext {
            kind: "series".into(),
            id: Some("Next".into()),
        };
        for path in ["unknown-episode.mp4", "unknown-season.mp4"] {
            assert!(
                s.next_video(&p.id, &id(path), &context)
                    .unwrap()
                    .media
                    .is_none()
            );
        }
        let next = s.next_video(&p.id, &id("known.mp4"), &context).unwrap();
        assert_eq!(next.media.unwrap().id, id("new.mp4"));
        assert!(next.new_season);
        assert!(next.available);
    }
    #[test]
    fn next_flat_numbered_series_uses_source_ordinals_without_rewriting_metadata() {
        let mut s = store();
        let items = ["010 Last.avi", "001 First.avi", "002 Second.avi"].map(|file| {
            let mut item = media(file, &format!("/nas/Flat/{file}"));
            item.category = "serie".into();
            item.series = Some("Flat".into());
            item.tmdb_id = Some(1234);
            item
        });
        s.snapshot(items.to_vec()).unwrap();
        let p = s.create_profile("Flat").unwrap();
        let found = s.catalog(&p.id).unwrap();
        let id = |name: &str| found.iter().find(|m| m.title == name).unwrap().id.clone();
        let context = PlaybackContext {
            kind: "series".into(),
            id: Some("tmdb:tv:1234".into()),
        };
        let next = s.next_video(&p.id, &id("001 First.avi"), &context).unwrap();
        assert_eq!(next.media.unwrap().id, id("002 Second.avi"));
        assert!(!next.order_unknown);
        assert!(!next.new_season);
        assert_eq!(
            s.next_video(&p.id, &id("002 Second.avi"), &context)
                .unwrap()
                .media
                .unwrap()
                .id,
            id("010 Last.avi")
        );
        let last = s.next_video(&p.id, &id("010 Last.avi"), &context).unwrap();
        assert!(last.media.is_none());
        assert!(!last.order_unknown);
        assert!(
            s.catalog(&p.id)
                .unwrap()
                .iter()
                .all(|m| m.season.is_none() && m.episode.is_none())
        );
        // Preserve a missing successor rather than silently skipping it.
        s.snapshot(vec![items[0].clone(), items[1].clone()])
            .unwrap();
        let next = s.next_video(&p.id, &id("001 First.avi"), &context).unwrap();
        assert_eq!(next.media.unwrap().id, id("002 Second.avi"));
        assert!(!next.available);
    }
    #[test]
    fn next_ambiguous_numbering_is_not_a_confirmed_end_of_series() {
        for files in [
            vec!["001 First.avi", "001 Duplicate.avi"],
            vec!["First.avi", "Second.avi"],
            vec!["2020 Film.avi", "2021 Film.avi"],
        ] {
            let mut s = store();
            let items = files
                .into_iter()
                .map(|file| {
                    let mut item = media(file, &format!("/nas/Ambiguous/{file}"));
                    item.category = "serie".into();
                    item.series = Some("Ambiguous".into());
                    item
                })
                .collect();
            s.snapshot(items).unwrap();
            let p = s.create_profile("Ambiguous").unwrap();
            let id = s
                .catalog(&p.id)
                .unwrap()
                .into_iter()
                .find(|m| m.category == "serie")
                .unwrap()
                .id;
            let next = s
                .next_video(
                    &p.id,
                    &id,
                    &PlaybackContext {
                        kind: "series".into(),
                        id: Some(format!("media:{id}")),
                    },
                )
                .unwrap();
            assert!(next.media.is_none());
            assert!(next.order_unknown);
        }
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
    fn short_watch_requires_server_tail_proof_at_end_and_manual_false_stays_effective() {
        let mut s = store();
        let p = s.create_profile("Short").unwrap();
        let id = s.catalog(&p.id).unwrap()[0].id.clone();
        let mut event = Progress {
            media_revision: 1,
            session_id: "short".into(),
            generation: 1,
            media_id: id.clone(),
            sequence: 1,
            position_ms: 59_000,
            duration_ms: 60_000,
            speed: 2.0,
            final_elapsed_ms: 2_500,
            playing: true,
            buffering: false,
            seek: false,
        };
        assert!(
            !s.save_validated_progress_with_short_tail(&p.id, &id, &event, true)
                .unwrap()
                .watched
        );
        event.sequence += 1;
        event.position_ms = 60_000;
        event.final_elapsed_ms = 500;
        assert!(
            !s.save_validated_progress(&p.id, &id, &event)
                .unwrap()
                .watched
        );
        event.sequence += 1;
        event.final_elapsed_ms = 0;
        event.playing = false;
        assert!(
            s.save_validated_progress_with_short_tail(&p.id, &id, &event, true)
                .unwrap()
                .watched
        );
        s.set_watched(&p.id, &id, false).unwrap();
        event.sequence += 1;
        assert!(
            !s.save_validated_progress_with_short_tail(&p.id, &id, &event, false)
                .unwrap()
                .watched
        );
        let saved = s.personal_state(&p.id, &id).unwrap();
        assert_eq!(saved.position_ms, 60_000);
        assert_eq!(saved.final_watched_ms, 0);
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
    fn measured_progress_is_exact_profile_scoped_and_independent_of_catalogue_sync() {
        let mut s = store();
        let p = s.create_profile("Progress").unwrap();
        let other = s.create_profile("Other").unwrap();
        let id = s.catalog(&p.id).unwrap()[0].id.clone();
        // A pre-existing resume has no measured duration: never infer from metadata.
        s.personal.execute("INSERT INTO cc_profile_state(profile_id,stable_media_id,position_ms) VALUES(?1,?2,5000)",params![p.id,id]).unwrap();
        assert_eq!(
            s.playback_progress(&p.id, None).unwrap()["items"],
            json!([])
        );
        let before = s.sync_revision(&p.id).unwrap();
        let mut event = Progress {
            media_id: id.clone(),
            media_revision: 1,
            session_id: "measured".into(),
            generation: 1,
            sequence: 1,
            position_ms: 5000,
            duration_ms: 100000,
            speed: 1.0,
            final_elapsed_ms: 0,
            playing: true,
            buffering: false,
            seek: false,
        };
        s.save_validated_progress(&p.id, &id, &event).unwrap();
        assert_eq!(
            s.sync_revision(&p.id).unwrap(),
            before,
            "a playhead must not invalidate the catalogue"
        );
        let result = s.playback_progress(&p.id, None).unwrap();
        assert_eq!(
            result["items"][0],
            json!({"media_id":id,"position_ms":5000,"duration_ms":100000,"watched":false})
        );
        assert_eq!(
            s.playback_progress(&other.id, None).unwrap()["items"],
            json!([])
        );
        assert!(s.playback_progress("missing", None).is_err());
        assert_eq!(
            s.playback_progress(&p.id, result["revision"].as_str())
                .unwrap()["unchanged"],
            true
        );
        event.position_ms = 42000;
        event.sequence += 1;
        event.playing = false;
        s.save_validated_progress(&p.id, &id, &event).unwrap();
        let moved = s
            .playback_progress(&p.id, result["revision"].as_str())
            .unwrap();
        assert_eq!(moved["unchanged"], false);
        assert_eq!(moved["items"][0]["position_ms"], 42000);
        let path = std::path::PathBuf::from(s.personal.path().unwrap());
        let reopened = Store::open(&path).unwrap();
        assert_eq!(
            reopened.playback_progress(&p.id, None).unwrap()["items"][0]["duration_ms"],
            100000
        );
        drop(reopened);
        event.duration_ms = 200000; // replay must not corrupt a previously measured duration
        assert!(s.save_validated_progress(&p.id, &id, &event).is_err());
        assert_eq!(
            s.playback_progress(&p.id, None).unwrap()["items"][0]["duration_ms"],
            100000
        );
        s.set_watched(&p.id, &id, true).unwrap();
        assert_eq!(
            s.playback_progress(&p.id, None).unwrap()["items"][0]["watched"],
            true
        );
        s.delete_profile(&p.id).unwrap();
        assert_eq!(
            s.personal
                .query_row("SELECT COUNT(*) FROM cc_resume_measurement", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
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

    #[test]
    fn verified_missing_file_marks_only_its_existing_catalogue_row_unavailable() {
        let mut s = store();
        let profile = s.create_profile("A").unwrap();
        let before_epoch = s.catalog_revision().unwrap();
        let item = s.catalog(&profile.id).unwrap().pop().unwrap();
        let before_revision = s.media_revision(&item.id).unwrap();
        assert!(
            s.mark_media_unavailable_at_path(
                &profile.id,
                &item.id,
                &s.media_path(&item.id).unwrap()
            )
            .unwrap()
        );
        assert!(!s.media(&profile.id, &item.id).unwrap().available);
        assert_eq!(s.media_revision(&item.id).unwrap(), before_revision + 1);
        assert!(s.catalog_revision().unwrap() > before_epoch);
        // Replaying the confirmed signal is idempotent and does not manufacture
        // a new revision or alter any path/source identity.
        assert!(
            !s.mark_media_unavailable_at_path(
                &profile.id,
                &item.id,
                &s.media_path(&item.id).unwrap()
            )
            .unwrap()
        );
        assert_eq!(s.media_revision(&item.id).unwrap(), before_revision + 1);
        assert_eq!(s.media(&profile.id, &item.id).unwrap().path, item.path);
    }

    #[test]
    fn verified_missing_file_rejects_a_reassociated_path_without_mutation() {
        let mut s = store();
        let profile = s.create_profile("A").unwrap();
        let item = s.catalog(&profile.id).unwrap().pop().unwrap();
        let before = s.media_revision(&item.id).unwrap();
        assert!(
            !s.mark_media_unavailable_at_path(&profile.id, &item.id, Path::new("/other/file.mkv"))
                .unwrap()
        );
        assert!(s.media(&profile.id, &item.id).unwrap().available);
        assert_eq!(s.media_revision(&item.id).unwrap(), before);
    }

    #[test]
    fn verified_missing_file_rolls_back_when_revision_update_fails() {
        let mut s = store();
        let profile = s.create_profile("A").unwrap();
        let item = s.catalog(&profile.id).unwrap().pop().unwrap();
        s.personal
            .execute_batch(
                "CREATE TRIGGER fail_missing_revision BEFORE UPDATE ON cc_media_revision BEGIN SELECT RAISE(ABORT,'fixture'); END;",
            )
            .unwrap();
        assert!(
            s.mark_media_unavailable_at_path(
                &profile.id,
                &item.id,
                &s.media_path(&item.id).unwrap()
            )
            .is_err()
        );
        assert!(s.media(&profile.id, &item.id).unwrap().available);
    }
}
