use crate::model::{
    Collection, Media, MediaPatch, SeriesPatch, SeriesProvider, Settings, merge_association,
    merge_enrichment, now,
};

/// Changing the derived series key must also bump the cache rule in `Store`.
/// It is deliberately separate from the database schema: this is a rebuildable
/// index, never source media data.
pub const SERIES_GROUP_RULE_VERSION: i64 = 2;

/// A pre-identification series is grouped by its source folder rather than by
/// title alone.  This is intentionally conservative: two "Doctor Who" trees
/// must never become one just because their display name is identical.  The
/// usual `Season 01` leaf is ignored so episodes of separate seasons remain
/// in the same source group.
fn legacy_series_root(media: &Media) -> String {
    let path = Path::new(&media.path);
    let mut parent = path.parent().unwrap_or(path);
    let leaf = parent.file_name().and_then(|v| v.to_str()).unwrap_or("");
    let normalized = crate::scanner::normalized(leaf).replace(['_', '.'], " ");
    if normalized.starts_with("season ")
        || normalized.starts_with("saison ")
        || normalized.starts_with('s') && normalized[1..].chars().all(|c| c.is_ascii_digit())
    {
        parent = parent.parent().unwrap_or(parent);
    }
    parent.to_string_lossy().to_string()
}

/// Match display spelling differences without making punctuation or a path
/// boundary disappear.  In particular, `Série Été` and `serie ete` are one
/// label, whereas two different parent trees remain two unknown series.
pub fn series_title_key(value: &str) -> String {
    crate::scanner::normalized(value)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn series_root_key(media: &Media) -> String {
    legacy_series_root(media)
        .split(std::path::MAIN_SEPARATOR)
        .map(series_title_key)
        .collect::<Vec<_>>()
        .join("\u{1f}")
}

fn series_label_key(media: &Media) -> String {
    series_title_key(media.series.as_deref().unwrap_or(&media.title))
}

fn same_series_group(anchor: &Media, candidate: &Media) -> bool {
    if anchor.category != "serie" || candidate.category != "serie" {
        return false;
    }
    match anchor.tmdb_id {
        Some(id) => candidate.tmdb_id == Some(id),
        None => {
            candidate.tmdb_id.is_none()
                && series_label_key(anchor) == series_label_key(candidate)
                && series_root_key(anchor) == series_root_key(candidate)
        }
    }
}

/// Internal, persisted grouping key for an unrecognised show. It is never
/// serialized to a client; callers expose an opaque stable anchor instead.
pub fn series_source_key(media: &Media) -> Option<String> {
    (media.category == "serie").then(|| {
        format!(
            "{}\u{1f}{}",
            series_label_key(media),
            series_root_key(media)
        )
    })
}
use rusqlite::{Connection, OptionalExtension, params};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

pub struct Database {
    connection: Connection,
}
fn db_error(_: rusqlite::Error) -> String {
    "Erreur de base locale. Vos données n’ont pas été remplacées.".into()
}

fn inherit_series_metadata(original: &Media, media: &mut Media, all: &[Media]) {
    if media.category == "serie"
        && (media.series != original.series || original.category != "serie")
        && let Some(target) = all.iter().find(|m| {
            m.id != media.id
                && m.category == "serie"
                && same_series_group(media, m)
                && m.series_metadata.is_some()
        })
    {
        media.series_metadata = target.series_metadata.clone();
        media.series_overview = target.series_overview.clone();
    }
}

impl Database {
    /// A frozen backup is never migrated or written by an export reader.
    pub fn open_readonly(path: &Path) -> Result<Self, String> {
        let connection =
            Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(db_error)?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(db_error)?;
        Ok(Self { connection })
    }
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(db_error)?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(db_error)?;
        connection
            .execute_batch(
                "PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
            CREATE TABLE IF NOT EXISTS media (id TEXT PRIMARY KEY, data TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS collections (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, kind TEXT NOT NULL, UNIQUE(kind,name));
            CREATE TABLE IF NOT EXISTS collection_members (collection_id INTEGER NOT NULL, media_id TEXT NOT NULL, PRIMARY KEY(collection_id,media_id));
            CREATE TABLE IF NOT EXISTS imported_sagas (media_id TEXT NOT NULL, name TEXT NOT NULL, PRIMARY KEY(media_id,name));
            CREATE TABLE IF NOT EXISTS collection_artwork (collection_id INTEGER PRIMARY KEY, poster TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS collection_positions (collection_id INTEGER NOT NULL, media_id TEXT NOT NULL, position INTEGER NOT NULL CHECK(position>=0), PRIMARY KEY(collection_id,media_id));
            -- Common collections predate the API revision contract.  Keep the
            -- revision beside them in the same SQLite database so every CAS
            -- mutation can be one IMMEDIATE transaction.
            CREATE TABLE IF NOT EXISTS collection_revisions (collection_id INTEGER PRIMARY KEY REFERENCES collections(id) ON DELETE CASCADE, revision INTEGER NOT NULL DEFAULT 1);
            INSERT OR IGNORE INTO collection_revisions(collection_id,revision) SELECT id,1 FROM collections;
            CREATE TRIGGER IF NOT EXISTS remove_collection_position AFTER DELETE ON collection_members BEGIN DELETE FROM collection_positions WHERE collection_id=OLD.collection_id AND media_id=OLD.media_id; END;
            ",
            )
            .map_err(db_error)?;
        // The compatibility catalogue shares a database with Chill Center;
        // never downgrade the latter's schema marker while opening a reader.
        let version: u32 = connection
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(db_error)?;
        if version == 0 {
            connection
                .pragma_update(None, "user_version", 1)
                .map_err(db_error)?;
        }
        Ok(Self { connection })
    }

    pub fn list(&self) -> Result<Vec<Media>, String> {
        let mut stmt = self
            .connection
            .prepare("SELECT data FROM media ORDER BY id")
            .map_err(db_error)?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(db_error)?;
        rows.map(|r| {
            serde_json::from_str(&r.map_err(db_error)?)
                .map_err(|_| "Catalogue local endommagé : aucune donnée remplacée.".into())
        })
        .collect()
    }

    pub fn get(&self, id: &str) -> Result<Media, String> {
        let data: String = self
            .connection
            .query_row("SELECT data FROM media WHERE id=?1", [id], |r| r.get(0))
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    "Vidéo introuvable dans le catalogue.".to_string()
                }
                error => db_error(error),
            })?;
        serde_json::from_str(&data).map_err(|_| db_error(rusqlite::Error::InvalidQuery))
    }

    pub fn put(&self, media: &Media) -> Result<(), String> {
        let json = serde_json::to_string(media).map_err(|_| "Métadonnées invalides.")?;
        self.connection.execute("INSERT INTO media(id,data) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET data=excluded.data", params![media.id, json]).map_err(db_error)?;
        Ok(())
    }

    pub fn patch(&self, id: &str, patch: &MediaPatch) -> Result<Media, String> {
        let tx = rusqlite::Transaction::new_unchecked(
            &self.connection,
            rusqlite::TransactionBehavior::Immediate,
        )
        .map_err(db_error)?;
        let mut media = self.get(id)?;
        let original = media.clone();
        patch.apply(&mut media)?;
        if media.series != original.series || media.category != original.category {
            inherit_series_metadata(&original, &mut media, &self.list()?);
        }
        tx.execute(
            "UPDATE media SET data=?1 WHERE id=?2",
            params![
                serde_json::to_string(&media).map_err(|_| "Métadonnées invalides.")?,
                media.id
            ],
        )
        .map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        Ok(media)
    }

    /// Applies an explicitly selected or refreshed provider record only when the
    /// item still equals the snapshot that was sent to TMDB.  This is deliberately
    /// separate from ordinary patches: a slow provider response must never win
    /// over a local correction or a personal poster import.
    pub fn apply_provider(
        &self,
        original: &Media,
        downloaded: Media,
        explicit_association: bool,
    ) -> Result<Media, String> {
        let tx = rusqlite::Transaction::new_unchecked(
            &self.connection,
            rusqlite::TransactionBehavior::Immediate,
        )
        .map_err(db_error)?;
        let latest = self.get(&original.id)?;
        let merged = if explicit_association {
            merge_association(original, &latest, downloaded)
        } else {
            merge_enrichment(original, &latest, downloaded)
        };
        // A provider refresh is only safe if fields it owns have not been
        // changed after the snapshot. `merge_*` preserves unrelated changes;
        // reject the request if it would silently discard a provider field.
        if latest.tmdb_id != original.tmdb_id
            || latest.poster != original.poster
            || latest.poster_custom != original.poster_custom
            || (!explicit_association
                && (latest.title != original.title
                    || latest.series != original.series
                    || latest.overview != original.overview
                    || latest.year != original.year
                    || latest.genres != original.genres))
        {
            return Err("La fiche a changé pendant la requête. Vos corrections sont conservées ; relancez l’opération.".into());
        }
        tx.execute(
            "UPDATE media SET data=?1 WHERE id=?2",
            params![
                serde_json::to_string(&merged).map_err(|_| "Métadonnées invalides.")?,
                merged.id
            ],
        )
        .map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        Ok(merged)
    }

    /// Commit a downloaded provider poster only after success, preserving a concurrent import.
    pub fn restore_poster(
        &self,
        original: &Media,
        poster: Option<String>,
    ) -> Result<Media, String> {
        let poster = poster
            .ok_or("TMDB n’a pas fourni d’affiche. Votre affiche personnelle est conservée.")?;
        let mut latest = self.get(&original.id)?;
        if latest.poster != original.poster
            || latest.poster_custom != original.poster_custom
            || latest.tmdb_id != original.tmdb_id
        {
            return Err(
                "L’affiche a été modifiée pendant la requête. Votre choix est conservé.".into(),
            );
        }
        latest.poster = Some(poster);
        latest.poster_custom = false;
        self.put(&latest)?;
        Ok(latest)
    }

    pub fn set_series_overview(
        &mut self,
        series: &str,
        overview: &str,
    ) -> Result<Vec<Media>, String> {
        if series.trim().is_empty() || series.len() > 500 || overview.len() > 20000 {
            return Err("Série ou synopsis invalide.".into());
        }
        let members: Vec<_> = self
            .list()?
            .into_iter()
            .filter(|m| m.category == "serie" && m.series.as_deref().unwrap_or(&m.title) == series)
            .map(|m| {
                (
                    m.id,
                    MediaPatch {
                        series_overview: Some(overview.into()),
                        ..Default::default()
                    },
                )
            })
            .collect();
        if members.is_empty() {
            return Err("Série introuvable dans le catalogue.".into());
        }
        self.apply_changes(&members)
    }

    /// Correct shared series fields atomically; never overwrite episode-specific fields.
    pub fn update_series(
        &mut self,
        series: &str,
        patch: &SeriesPatch,
    ) -> Result<Vec<Media>, String> {
        if series.trim().is_empty() || series.len() > 500 {
            return Err("Série invalide.".into());
        }
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(db_error)?;
        let all: Vec<Media> = {
            let mut stmt = tx
                .prepare("SELECT data FROM media ORDER BY id")
                .map_err(db_error)?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(db_error)?;
            rows.map(|row| {
                serde_json::from_str(&row.map_err(db_error)?)
                    .map_err(|_| "Entrée de catalogue invalide.".to_owned())
            })
            .collect::<Result<_, _>>()?
        };
        let name = patch.name.trim();
        if name != series
            && all
                .iter()
                .any(|m| m.category == "serie" && m.series.as_deref().unwrap_or(&m.title) == name)
        {
            return Err("Une série porte déjà ce nom. Réassociez les épisodes individuellement pour les regrouper.".into());
        }
        let mut members: Vec<_> = all
            .into_iter()
            .filter(|m| m.category == "serie" && m.series.as_deref().unwrap_or(&m.title) == series)
            .collect();
        let metadata = patch.validate(
            members
                .first()
                .ok_or("Série introuvable dans le catalogue.")?,
        )?;
        for member in &mut members {
            member.series = Some(name.into());
            member.series_manual = true;
            member.series_overview = metadata.overview.clone();
            member.series_metadata = Some(metadata.clone());
            tx.execute(
                "UPDATE media SET data=?1 WHERE id=?2",
                params![
                    serde_json::to_string(member).map_err(|_| "Métadonnées invalides.")?,
                    member.id
                ],
            )
            .map_err(db_error)?;
        }
        tx.commit().map_err(db_error)?;
        Ok(members)
    }

    pub fn last_scan(&self) -> Option<i64> {
        self.connection
            .query_row(
                "SELECT value FROM settings WHERE key='last_scan'",
                [],
                |r| r.get::<_, String>(0),
            )
            .ok()?
            .parse()
            .ok()
    }

    pub fn series_members(&self, series: &str) -> Result<Vec<Media>, String> {
        if series.trim().is_empty() || series.len() > 500 {
            return Err("Série invalide.".into());
        }
        let members: Vec<_> = self
            .list()?
            .into_iter()
            .filter(|m| m.category == "serie" && m.series.as_deref().unwrap_or(&m.title) == series)
            .collect();
        if members.is_empty() {
            return Err("Série introuvable dans le catalogue.".into());
        }
        Ok(members)
    }

    /// Resolve a group from a concrete episode.  Public endpoints should use
    /// this rather than a display name whenever a stable media id is available.
    pub fn series_members_for(&self, anchor: &Media) -> Result<Vec<Media>, String> {
        let members: Vec<_> = self
            .list()?
            .into_iter()
            .filter(|candidate| same_series_group(anchor, candidate))
            .collect();
        if members.is_empty() {
            return Err("Série introuvable dans le catalogue.".into());
        }
        Ok(members)
    }

    /// Commit shared provider fields after download, only if the series association
    /// and its shared fields have not changed meanwhile. Episode edits stay intact.
    pub fn apply_series_provider(
        &self,
        series: &str,
        originals: &[Media],
        provider: &SeriesProvider,
        explicit: bool,
    ) -> Result<Vec<Media>, String> {
        self.apply_series_provider_with(
            series,
            originals,
            provider,
            explicit,
            |_| Ok(()),
            |_, _| Ok(()),
        )
    }
    /// The shared catalogue and its caller's revision rows live in the same
    /// database. Publish both in this transaction, never as two commits.
    pub fn apply_series_provider_with(
        &self,
        series: &str,
        originals: &[Media],
        provider: &SeriesProvider,
        explicit: bool,
        before_write: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<(), String>,
        after_write: impl FnOnce(&rusqlite::Transaction<'_>, &[Media]) -> Result<(), String>,
    ) -> Result<Vec<Media>, String> {
        let sample = originals.first().ok_or("Série introuvable.")?;
        let metadata = SeriesPatch {
            name: provider.name.clone(),
            year: provider.metadata.year.unwrap_or(0),
            genres: provider.metadata.genres.clone(),
            overview: provider.metadata.overview.clone(),
            cast: provider.metadata.cast.clone(),
            directors: provider.metadata.directors.clone(),
        }
        .validate(sample)?;
        if provider.tmdb_id <= 0 {
            return Err("Identifiant TMDB invalide.".into());
        }
        let tx = rusqlite::Transaction::new_unchecked(
            &self.connection,
            rusqlite::TransactionBehavior::Immediate,
        )
        .map_err(db_error)?;
        before_write(&tx)?;
        let mut members = Self::checked_series_members(&tx, originals)?;
        let keep_name = !explicit && members.iter().any(|m| m.series_manual);
        let name = if keep_name {
            series
        } else {
            provider.name.trim()
        };
        // A title collision is not an identity collision: the selected TMDB id
        // is persisted on only this frozen member set and is used for later
        // grouping.  Refusing it made correctly selected homonymous shows
        // impossible to catalogue.
        for member in &mut members {
            let association_changed = member.tmdb_id != Some(provider.tmdb_id);
            member.series = Some(name.into());
            member.series_manual = keep_name;
            member.series_metadata = Some(metadata.clone());
            member.series_overview = metadata.overview.clone();
            member.tmdb_id = Some(provider.tmdb_id);
            if !member.poster_custom && (association_changed || provider.poster.is_some()) {
                member.poster = provider.poster.clone();
            }
            tx.execute(
                "UPDATE media SET data=?1 WHERE id=?2",
                params![
                    serde_json::to_string(member).map_err(|_| "Métadonnées invalides.")?,
                    member.id
                ],
            )
            .map_err(db_error)?;
        }
        after_write(&tx, &members)?;
        tx.commit().map_err(db_error)?;
        Ok(members)
    }

    /// Re-resolve the complete group while the caller holds its write reservation.
    /// Checking captured ids alone misses newly scanned episodes of this group.
    pub fn checked_series_members(
        tx: &rusqlite::Transaction<'_>,
        originals: &[Media],
    ) -> Result<Vec<Media>, String> {
        let sample = originals.first().ok_or("Série introuvable.")?;
        let mut statement = tx.prepare("SELECT data FROM media").map_err(db_error)?;
        let all = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(db_error)?
            .map(|row| {
                serde_json::from_str::<Media>(&row.map_err(db_error)?)
                    .map_err(|_| "Catalogue local endommagé.".to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let anchor = all
            .iter()
            .find(|item| item.id == sample.id)
            .ok_or("Série introuvable.")?;
        let members: Vec<_> = all
            .iter()
            .filter(|item| same_series_group(anchor, item))
            .cloned()
            .collect();
        if members.len() != originals.len()
            || members.iter().any(|m| {
                !originals.iter().any(|o| {
                    o.id == m.id
                        && o.series == m.series
                        && o.category == m.category
                        && o.series_manual == m.series_manual
                        && o.series_metadata == m.series_metadata
                        && o.series_overview == m.series_overview
                        && o.tmdb_id == m.tmdb_id
                        && o.poster == m.poster
                        && o.poster_custom == m.poster_custom
                })
            })
        {
            return Err("La série a changé pendant la recherche. Vos corrections sont conservées ; relancez l’opération.".into());
        }
        Ok(members)
    }

    /// Forget only confirmed-unavailable catalogue entries. Never accesses library files.
    pub fn remove_unavailable(&mut self, ids: &[String]) -> Result<Vec<String>, String> {
        if ids.is_empty() || ids.len() > 200_000 {
            return Err("Sélection vide ou trop volumineuse.".into());
        }
        // Reserve the SQLite writer before validation, including across app instances.
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(db_error)?;
        let mut seen = HashSet::new();
        for id in ids {
            if !seen.insert(id) {
                return Err("Sélection invalide : vidéo dupliquée.".into());
            }
            let data: String = tx
                .query_row("SELECT data FROM media WHERE id=?1", [id], |r| r.get(0))
                .map_err(|_| "Vidéo introuvable dans le catalogue.".to_string())?;
            let media: Media =
                serde_json::from_str(&data).map_err(|_| "Entrée de catalogue invalide.")?;
            if media.available {
                return Err("Une vidéo est redevenue disponible. Actualisez la liste ; aucune référence supprimée.".into());
            }
        }
        for id in ids {
            tx.execute("DELETE FROM collection_members WHERE media_id=?1", [id])
                .map_err(db_error)?;
            tx.execute("DELETE FROM imported_sagas WHERE media_id=?1", [id])
                .map_err(db_error)?;
            tx.execute("DELETE FROM media WHERE id=?1", [id])
                .map_err(db_error)?;
        }
        tx.commit().map_err(db_error)?;
        Ok(ids.to_vec())
    }

    /// Import each historical single-saga association once. Removals remain durable.
    pub fn collections(&mut self) -> Result<Vec<Collection>, String> {
        let all = self.list()?;
        let tx = self.connection.transaction().map_err(db_error)?;
        for media in all.iter().filter(|m| m.category == "film") {
            let Some(name) = media
                .saga
                .as_deref()
                .map(str::trim)
                .filter(|n| !n.is_empty())
            else {
                continue;
            };
            // Keep malformed legacy metadata intact without importing unbounded collections.
            if name.len() > 200 {
                continue;
            }
            let exists: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM collections WHERE name=?1 AND kind='saga')",
                    [name],
                    |r| r.get(0),
                )
                .map_err(db_error)?;
            let count: i64 = tx
                .query_row("SELECT COUNT(*) FROM collections", [], |r| r.get(0))
                .map_err(db_error)?;
            if !exists && count >= 1000 {
                continue;
            }
            if tx
                .execute(
                    "INSERT OR IGNORE INTO imported_sagas(media_id,name) VALUES(?1,?2)",
                    params![media.id, name],
                )
                .map_err(db_error)?
                == 1
            {
                tx.execute(
                    "INSERT OR IGNORE INTO collections(name,kind) VALUES(?1,'saga')",
                    [name],
                )
                .map_err(db_error)?;
                tx.execute("INSERT OR IGNORE INTO collection_members(collection_id,media_id) SELECT id,?1 FROM collections WHERE kind='saga' AND name=?2", params![media.id,name]).map_err(db_error)?;
            }
        }
        // A film recategorized as an episode must no longer belong to film-only sagas.
        for media in all.iter().filter(|m| m.category != "film") {
            tx.execute("DELETE FROM collection_members WHERE media_id=?1 AND collection_id IN (SELECT id FROM collections WHERE kind='saga')", [&media.id]).map_err(db_error)?;
        }
        // Legacy/TMDB imports above create collections after Database::open's
        // migration. Initialize their CAS in the same transaction, without
        // ever resetting a revision already issued to a client.
        tx.execute("INSERT OR IGNORE INTO collection_revisions(collection_id,revision) SELECT id,1 FROM collections", []).map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        let mut stmt = self
            .connection
            .prepare("SELECT id,name,kind FROM collections ORDER BY name,id")
            .map_err(db_error)?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })
            .map_err(db_error)?;
        let mut result = Vec::new();
        for row in rows {
            let (id, name, kind) = row.map_err(db_error)?;
            let media_ids = self.collection_member_ids(id)?;
            result.push(Collection {
                id: id.to_string(),
                name,
                kind,
                media_ids,
                poster: self
                    .connection
                    .query_row(
                        "SELECT poster FROM collection_artwork WHERE collection_id=?1",
                        [id],
                        |r| r.get::<_, String>(0),
                    )
                    .optional()
                    .map_err(db_error)?,
            });
        }
        Ok(result)
    }

    pub fn create_collection(&mut self, name: &str, kind: &str) -> Result<Vec<Collection>, String> {
        let name = name.trim();
        if name.is_empty() || name.len() > 200 || !["saga", "playlist"].contains(&kind) {
            return Err("Nom ou type de collection invalide.".into());
        }
        let count: i64 = self
            .connection
            .query_row("SELECT COUNT(*) FROM collections", [], |r| r.get(0))
            .map_err(db_error)?;
        if count >= 1000 {
            return Err("Limite de 1 000 collections atteinte.".into());
        }
        self.connection
            .execute(
                "INSERT INTO collections(name,kind) VALUES(?1,?2)",
                params![name, kind],
            )
            .map_err(|_| "Une collection de ce nom existe déjà.".to_string())?;
        let id = self.connection.last_insert_rowid();
        self.connection
            .execute(
                "INSERT INTO collection_revisions(collection_id,revision) VALUES(?1,1)",
                [id],
            )
            .map_err(db_error)?;
        self.collections()
    }

    pub fn saga_revision(&self, collection_id: &str) -> Result<u64, String> {
        let id: i64 = collection_id.parse().map_err(|_| "Saga introuvable.")?;
        self.connection
            .query_row(
                "SELECT revision FROM collection_revisions WHERE collection_id=?1",
                [id],
                |r| r.get::<_, i64>(0),
            )
            .optional()
            .map_err(db_error)?
            .map(|n| n as u64)
            .ok_or_else(|| "Saga introuvable.".into())
    }
    fn require_saga_revision(
        &self,
        tx: &rusqlite::Transaction<'_>,
        id: i64,
        expected: u64,
    ) -> Result<(), String> {
        let kind: String = tx
            .query_row("SELECT kind FROM collections WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .map_err(|_| "Saga introuvable.")?;
        if kind != "saga" {
            return Err("Cette action est réservée aux sagas.".into());
        }
        let changed=tx.execute("UPDATE collection_revisions SET revision=revision+1 WHERE collection_id=?1 AND revision=?2",params![id,expected as i64]).map_err(db_error)?;
        if changed != 1 {
            return Err("La saga a changé entre-temps.".into());
        }
        Ok(())
    }
    pub fn rename_saga_at_revision(
        &mut self,
        id: &str,
        name: &str,
        expected: u64,
    ) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() || name.len() > 200 {
            return Err("Nom de saga invalide (200 caractères maximum).".into());
        };
        let id: i64 = id.parse().map_err(|_| "Saga introuvable.")?;
        let tx = rusqlite::Transaction::new_unchecked(
            &self.connection,
            rusqlite::TransactionBehavior::Immediate,
        )
        .map_err(db_error)?;
        self.require_saga_revision(&tx, id, expected)?;
        if tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM collections WHERE kind='saga' AND name=?1 AND id<>?2)",
                params![name, id],
                |r| r.get::<_, bool>(0),
            )
            .map_err(db_error)?
        {
            return Err("Une saga de ce nom existe déjà.".into());
        };
        tx.execute(
            "UPDATE collections SET name=?1 WHERE id=?2",
            params![name, id],
        )
        .map_err(db_error)?;
        tx.commit().map_err(db_error)
    }
    pub fn saga_members_at_revision(
        &mut self,
        id: &str,
        media_ids: &[String],
        expected: u64,
        add: bool,
    ) -> Result<(), String> {
        if media_ids.is_empty() || media_ids.len() > 1000 {
            return Err("Sélectionnez de 1 à 1 000 vidéos par ajout.".into());
        };
        let idn: i64 = id.parse().map_err(|_| "Saga introuvable.")?;
        let tx = rusqlite::Transaction::new_unchecked(
            &self.connection,
            rusqlite::TransactionBehavior::Immediate,
        )
        .map_err(db_error)?;
        self.require_saga_revision(&tx, idn, expected)?;
        let mut seen = HashSet::new();
        for media in media_ids {
            if !seen.insert(media) {
                return Err("Sélection invalide : vidéo dupliquée.".into());
            };
            if add && self.get(media)?.category != "film" {
                return Err("Une saga ne peut contenir que des films.".into());
            }
        }
        for media in media_ids {
            if add {
                tx.execute("INSERT OR IGNORE INTO collection_members(collection_id,media_id) VALUES(?1,?2)",params![idn,media]).map_err(db_error)?;
            } else {
                tx.execute(
                    "DELETE FROM collection_members WHERE collection_id=?1 AND media_id=?2",
                    params![idn, media],
                )
                .map_err(db_error)?;
            }
        }
        tx.commit().map_err(db_error)
    }
    pub fn reorder_saga_at_revision(
        &mut self,
        id: &str,
        expected: u64,
        expected_order: &[String],
        desired: &[String],
    ) -> Result<(), String> {
        if expected_order.len() > 1000 || desired.len() > 1000 {
            return Err("Liste trop volumineuse.".into());
        };
        let idn: i64 = id.parse().map_err(|_| "Saga introuvable.")?;
        let tx = rusqlite::Transaction::new_unchecked(
            &self.connection,
            rusqlite::TransactionBehavior::Immediate,
        )
        .map_err(db_error)?;
        self.require_saga_revision(&tx, idn, expected)?;
        let current = self.collection_member_ids(idn)?;
        if current != expected_order {
            return Err("La saga a changé entre-temps.".into());
        };
        let set: HashSet<_> = desired.iter().collect();
        if desired.len() != current.len()
            || set.len() != current.len()
            || !current.iter().all(|m| set.contains(m))
        {
            return Err("Ordre invalide : conservez chaque film exactement une fois.".into());
        };
        tx.execute(
            "DELETE FROM collection_positions WHERE collection_id=?1",
            [idn],
        )
        .map_err(db_error)?;
        for (pos, m) in desired.iter().enumerate() {
            tx.execute("INSERT INTO collection_positions(collection_id,media_id,position) VALUES(?1,?2,?3)",params![idn,m,pos as i64]).map_err(db_error)?;
        }
        tx.commit().map_err(db_error)
    }
    pub fn delete_saga_at_revision(&mut self, id: &str, expected: u64) -> Result<(), String> {
        let id: i64 = id.parse().map_err(|_| "Saga introuvable.")?;
        let tx = rusqlite::Transaction::new_unchecked(
            &self.connection,
            rusqlite::TransactionBehavior::Immediate,
        )
        .map_err(db_error)?;
        self.require_saga_revision(&tx, id, expected)?;
        tx.execute("DELETE FROM collections WHERE id=?1", [id])
            .map_err(db_error)?;
        tx.commit().map_err(db_error)
    }

    pub fn saga(&self, collection_id: &str) -> Result<Option<String>, String> {
        let id: i64 = collection_id.parse().map_err(|_| "Saga introuvable.")?;
        let kind: String = self
            .connection
            .query_row("SELECT kind FROM collections WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .map_err(|_| "Saga introuvable.")?;
        if kind != "saga" {
            return Err("Cette action est réservée aux sagas.".into());
        }
        self.connection
            .query_row(
                "SELECT poster FROM collection_artwork WHERE collection_id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()
            .map_err(db_error)
    }

    pub fn rename_saga(
        &mut self,
        collection_id: &str,
        name: &str,
    ) -> Result<Vec<Collection>, String> {
        let name = name.trim();
        if name.is_empty() || name.len() > 200 {
            return Err("Nom de saga invalide (200 caractères maximum).".into());
        }
        // Import historical membership before renaming, so old metadata cannot recreate it.
        self.collections()?;
        let tx = rusqlite::Transaction::new_unchecked(
            &self.connection,
            rusqlite::TransactionBehavior::Immediate,
        )
        .map_err(db_error)?;
        self.saga(collection_id)?;
        let id: i64 = collection_id.parse().map_err(|_| "Saga introuvable.")?;
        let collision: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM collections WHERE kind='saga' AND name=?1 AND id<>?2)",
                params![name, id],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        if collision {
            return Err("Une saga de ce nom existe déjà.".into());
        }
        tx.execute(
            "UPDATE collections SET name=?1 WHERE id=?2",
            params![name, id],
        )
        .map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        self.collections()
    }

    /// The path is trusted, produced by the bounded image importer/downloader, never IPC.
    /// Compare the previous selection after slow I/O to preserve a concurrent image edit.
    pub fn set_saga_poster(
        &mut self,
        collection_id: &str,
        expected: &Option<String>,
        poster: Option<String>,
    ) -> Result<Vec<Collection>, String> {
        let tx = rusqlite::Transaction::new_unchecked(
            &self.connection,
            rusqlite::TransactionBehavior::Immediate,
        )
        .map_err(db_error)?;
        if &self.saga(collection_id)? != expected {
            return Err(
                "L’image de la saga a changé entre-temps. Aucune modification appliquée.".into(),
            );
        }
        let id: i64 = collection_id.parse().map_err(|_| "Saga introuvable.")?;
        if let Some(poster) = poster {
            tx.execute("INSERT INTO collection_artwork(collection_id,poster) VALUES(?1,?2) ON CONFLICT(collection_id) DO UPDATE SET poster=excluded.poster", params![id,poster]).map_err(db_error)?;
        } else {
            tx.execute(
                "DELETE FROM collection_artwork WHERE collection_id=?1",
                [id],
            )
            .map_err(db_error)?;
        }
        tx.commit().map_err(db_error)?;
        self.collections()
    }

    pub fn use_saga_member_poster(
        &mut self,
        collection_id: &str,
        media_id: &str,
    ) -> Result<Vec<Collection>, String> {
        let tx = rusqlite::Transaction::new_unchecked(
            &self.connection,
            rusqlite::TransactionBehavior::Immediate,
        )
        .map_err(db_error)?;
        self.saga(collection_id)?;
        let id: i64 = collection_id.parse().map_err(|_| "Saga introuvable.")?;
        let member: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM collection_members WHERE collection_id=?1 AND media_id=?2)",
            params![id,media_id], |r| r.get(0),
        ).map_err(db_error)?;
        let media = self.get(media_id)?;
        if !member || media.category != "film" {
            return Err("Choisissez l’image d’un film appartenant à cette saga.".into());
        }
        let poster = media.poster.ok_or("Ce film n’a pas d’image.")?;
        tx.execute("INSERT INTO collection_artwork(collection_id,poster) VALUES(?1,?2) ON CONFLICT(collection_id) DO UPDATE SET poster=excluded.poster", params![id,poster]).map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        self.collections()
    }

    fn collection_member_ids(&self, id: i64) -> Result<Vec<String>, String> {
        let mut members = self.connection.prepare(
            "SELECT m.media_id FROM collection_members m LEFT JOIN collection_positions p ON p.collection_id=m.collection_id AND p.media_id=m.media_id WHERE m.collection_id=?1 ORDER BY CASE WHEN p.position IS NULL THEN 1 ELSE 0 END,p.position,m.rowid",
        ).map_err(db_error)?;
        members
            .query_map([id], |r| r.get::<_, String>(0))
            .map_err(db_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db_error)
    }

    pub fn add_collection_members(
        &mut self,
        collection_id: &str,
        media_ids: &[String],
    ) -> Result<Vec<Collection>, String> {
        if media_ids.is_empty() || media_ids.len() > 1000 {
            return Err("Sélectionnez de 1 à 1 000 vidéos par ajout.".into());
        }
        self.collections()?;
        let tx = rusqlite::Transaction::new_unchecked(
            &self.connection,
            rusqlite::TransactionBehavior::Immediate,
        )
        .map_err(db_error)?;
        let id: i64 = collection_id
            .parse()
            .map_err(|_| "Collection introuvable.")?;
        let kind: String = tx
            .query_row("SELECT kind FROM collections WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .map_err(|_| "Collection introuvable.")?;
        let mut seen = HashSet::new();
        for media_id in media_ids {
            if !seen.insert(media_id) {
                return Err("Sélection invalide : vidéo dupliquée.".into());
            }
            let media = self.get(media_id)?;
            if kind == "saga" && media.category != "film" {
                return Err("Une saga ne peut contenir que des films.".into());
            }
        }
        for media_id in media_ids {
            // Existing entries keep their position. New entries append in selection order.
            tx.execute(
                "INSERT OR IGNORE INTO collection_members(collection_id,media_id) VALUES(?1,?2)",
                params![id, media_id],
            )
            .map_err(db_error)?;
        }
        tx.commit().map_err(db_error)?;
        self.collections()
    }

    pub fn reorder_playlist(
        &mut self,
        collection_id: &str,
        expected_order: &[String],
        media_ids: &[String],
    ) -> Result<Vec<Collection>, String> {
        if expected_order.len() > 200_000 || media_ids.len() > 200_000 {
            return Err("Liste trop volumineuse.".into());
        }
        let tx = rusqlite::Transaction::new_unchecked(
            &self.connection,
            rusqlite::TransactionBehavior::Immediate,
        )
        .map_err(db_error)?;
        let id: i64 = collection_id.parse().map_err(|_| "Liste introuvable.")?;
        let kind: String = tx
            .query_row("SELECT kind FROM collections WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .map_err(|_| "Liste introuvable.")?;
        if kind != "playlist" {
            return Err("Seules les listes de lecture peuvent être réorganisées.".into());
        }
        let current = self.collection_member_ids(id)?;
        if current != expected_order {
            return Err(
                "La liste a changé entre-temps. Actualisez-la avant de la réorganiser.".into(),
            );
        }
        let desired: HashSet<_> = media_ids.iter().collect();
        if media_ids.len() != current.len()
            || desired.len() != current.len()
            || !current.iter().all(|member| desired.contains(member))
        {
            return Err(
                "Ordre invalide : conservez chaque vidéo de la liste exactement une fois.".into(),
            );
        }
        tx.execute(
            "DELETE FROM collection_positions WHERE collection_id=?1",
            [id],
        )
        .map_err(db_error)?;
        for (position, media_id) in media_ids.iter().enumerate() {
            tx.execute("INSERT INTO collection_positions(collection_id,media_id,position) VALUES(?1,?2,?3)", params![id,media_id,position as i64]).map_err(db_error)?;
        }
        tx.commit().map_err(db_error)?;
        self.collections()
    }

    pub fn set_collection_member(
        &mut self,
        collection_id: &str,
        media_id: &str,
        add: bool,
    ) -> Result<Vec<Collection>, String> {
        if add {
            return self.add_collection_members(collection_id, &[media_id.to_owned()]);
        }
        // Complete legacy import before the first edit, including removal.
        self.collections()?;
        let tx = rusqlite::Transaction::new_unchecked(
            &self.connection,
            rusqlite::TransactionBehavior::Immediate,
        )
        .map_err(db_error)?;
        let id: i64 = collection_id
            .parse()
            .map_err(|_| "Collection introuvable.")?;
        let _kind: String = self
            .connection
            .query_row("SELECT kind FROM collections WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .map_err(|_| "Collection introuvable.")?;
        self.get(media_id)?;
        tx.execute(
            "DELETE FROM collection_members WHERE collection_id=?1 AND media_id=?2",
            params![id, media_id],
        )
        .map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        self.collections()
    }

    /// Delete only the local collection and its associations, never a media record or file.
    pub fn delete_collection(&mut self, collection_id: &str) -> Result<Vec<Collection>, String> {
        let id: i64 = collection_id
            .parse()
            .map_err(|_| "Collection introuvable.")?;
        let tx = self.connection.transaction().map_err(db_error)?;
        if tx
            .execute("DELETE FROM collections WHERE id=?1", [id])
            .map_err(db_error)?
            != 1
        {
            return Err("Collection introuvable.".into());
        }
        tx.execute(
            "DELETE FROM collection_members WHERE collection_id=?1",
            [id],
        )
        .map_err(db_error)?;
        tx.execute(
            "DELETE FROM collection_artwork WHERE collection_id=?1",
            [id],
        )
        .map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        self.collections()
    }

    pub fn set_rating(&mut self, id: &str, rating: u8, scope: &str) -> Result<Vec<Media>, String> {
        if rating > 5 || !["video", "series", "season"].contains(&scope) {
            return Err("Note ou portée invalide.".into());
        }
        let target = self.get(id)?;
        if scope != "video" && target.category != "serie" {
            return Err("Cette portée est réservée aux séries.".into());
        }
        let key = target.series.as_deref().unwrap_or(&target.title);
        let changes: Vec<_> = self
            .list()?
            .into_iter()
            .filter(|m| {
                if scope == "video" {
                    m.id == id
                } else {
                    m.category == "serie"
                        && m.series.as_deref().unwrap_or(&m.title) == key
                        && (scope == "series" || m.season == target.season)
                }
            })
            .map(|m| {
                (
                    m.id,
                    MediaPatch {
                        rating: Some(rating),
                        ..Default::default()
                    },
                )
            })
            .collect();
        self.apply_changes(&changes)
    }

    pub fn settings(&self) -> Result<Settings, String> {
        let value = self.connection.query_row(
            "SELECT value FROM settings WHERE key='preferences'",
            [],
            |r| r.get::<_, String>(0),
        );
        match value {
            Ok(s) => serde_json::from_str(&s).map_err(|_| "Réglages locaux invalides.".into()),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(Settings::default()),
            Err(e) => Err(db_error(e)),
        }
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), String> {
        settings.validate()?;
        let mut safe = settings.clone();
        safe.tmdb_token.clear();
        safe.tmdb_configured = false;
        self.connection.execute("INSERT INTO settings(key,value) VALUES('preferences',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [serde_json::to_string(&safe).map_err(|_| "Réglages invalides.")?]).map_err(db_error)?;
        Ok(())
    }

    pub fn snapshot(&mut self, discovered: Vec<Media>) -> Result<i64, String> {
        self.snapshot_with(discovered, |_, _| Ok(()))
    }

    /// Extend a snapshot inside its transaction (e.g. stable identities). Any
    /// extension failure rolls back media, last_scan and the extension together.
    pub fn snapshot_with(
        &mut self,
        discovered: Vec<Media>,
        extend: impl FnOnce(&rusqlite::Transaction<'_>, &[Media]) -> Result<(), String>,
    ) -> Result<i64, String> {
        let previous = self.list()?;
        if discovered.is_empty() && previous.iter().any(|m| m.available) {
            return Err("Le dossier ne contient aucune vidéo. Par sécurité, le catalogue précédent est conservé. Vérifiez le montage NFS.".into());
        }
        let mut all: HashMap<String, Media> = previous
            .into_iter()
            .map(|mut m| {
                m.available = false;
                (m.id.clone(), m)
            })
            .collect();
        for new in discovered {
            if let Some(old) = all.get_mut(&new.id) {
                old.path = new.path;
                old.available = true;
            } else {
                all.insert(new.id.clone(), new);
            }
        }
        let all: Vec<Media> = all.into_values().collect();
        // Serialization/merge work stays outside the SQLite write lock.
        let serialized = all
            .iter()
            .map(|m| serde_json::to_string(m).map_err(|_| "Métadonnées invalides.".to_string()))
            .collect::<Result<Vec<_>, _>>()?;
        let time = now();
        let tx = self.connection.transaction().map_err(db_error)?;
        for (m, data) in all.iter().zip(serialized) {
            tx.execute("INSERT INTO media(id,data) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET data=excluded.data", params![m.id, data]).map_err(db_error)?;
        }
        tx.execute("INSERT INTO settings(key,value) VALUES('last_scan',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [time.to_string()]).map_err(db_error)?;
        extend(&tx, &all)?;
        tx.commit().map_err(db_error)?;
        Ok(time)
    }

    pub fn cancellation_probe(
        &self,
        probe: Option<std::sync::Arc<dyn Fn() -> bool + Send + Sync>>,
    ) -> Result<(), String> {
        match probe {
            Some(probe) => {
                let commit = probe.clone();
                self.connection
                    .commit_hook(Some(move || commit()))
                    .map_err(db_error)?;
                self.connection
                    .progress_handler(1000, Some(move || probe()))
                    .map_err(db_error)?;
            }
            None => {
                self.connection
                    .commit_hook(None::<fn() -> bool>)
                    .map_err(db_error)?;
                self.connection
                    .progress_handler(0, None::<fn() -> bool>)
                    .map_err(db_error)?;
            }
        }
        Ok(())
    }

    /// Validate all changes first, then commit the entire preview atomically.
    pub fn apply_changes(
        &mut self,
        changes: &[(String, MediaPatch)],
    ) -> Result<Vec<Media>, String> {
        let tx = rusqlite::Transaction::new_unchecked(
            &self.connection,
            rusqlite::TransactionBehavior::Immediate,
        )
        .map_err(db_error)?;
        let originals = self.list()?;
        let mut updated = HashMap::new();
        for (id, patch) in changes {
            if updated.contains_key(id) {
                return Err("Proposition invalide : vidéo dupliquée.".into());
            }
            let mut media = self.get(id)?;
            patch.apply(&mut media)?;
            updated.insert(id.clone(), media);
        }
        let effective: Vec<_> = originals
            .iter()
            .map(|m| updated.get(&m.id).unwrap_or(m).clone())
            .collect();
        for (id, media) in &mut updated {
            inherit_series_metadata(
                originals
                    .iter()
                    .find(|m| &m.id == id)
                    .ok_or("Vidéo introuvable.")?,
                media,
                &effective,
            );
        }
        for media in updated.values() {
            tx.execute(
                "UPDATE media SET data=?1 WHERE id=?2",
                params![
                    serde_json::to_string(media).map_err(|_| "Proposition invalide.")?,
                    media.id
                ],
            )
            .map_err(db_error)?;
        }
        tx.commit().map_err(db_error)?;
        self.list()
    }
}

pub fn similar(media: &Media, all: Vec<Media>) -> Vec<Media> {
    let mut scored: Vec<_> = all
        .into_iter()
        .filter(|m| m.id != media.id && m.category == media.category && m.available)
        .map(|m| {
            let genres = m.genres.iter().filter(|g| media.genres.contains(g)).count() as i32 * 10;
            let saga = if m.saga.is_some() && m.saga == media.saga {
                30
            } else {
                0
            };
            let year = match (m.year, media.year) {
                (Some(a), Some(b)) if (a - b).abs() <= 5 => 3,
                _ => 0,
            };
            (genres + saga + year, m)
        })
        .filter(|(score, _)| *score > 0)
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.title.cmp(&b.1.title)));
    scored.into_iter().take(12).map(|(_, m)| m).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lazily_imported_sagas_have_revisions_without_resetting_existing_cas() {
        let mut db = db();
        let mut film = movie("Auto.mkv");
        film.saga = Some("TMDB saga".into());
        db.put(&film).unwrap();
        let collection = db.collections().unwrap().remove(0);
        assert_eq!(db.saga_revision(&collection.id).unwrap(), 1);
        db.rename_saga_at_revision(&collection.id, "Renamed", 1)
            .unwrap();
        let revision = db.saga_revision(&collection.id).unwrap();
        db.collections().unwrap();
        assert_eq!(db.saga_revision(&collection.id).unwrap(), revision);
        // Repair old rows missing the additive revision, never media metadata.
        db.connection
            .execute(
                "DELETE FROM collection_revisions WHERE collection_id=?1",
                [&collection.id],
            )
            .unwrap();
        db.collections().unwrap();
        assert_eq!(db.saga_revision(&collection.id).unwrap(), 1);
        assert_eq!(db.get(&film.id).unwrap().saga.as_deref(), Some("TMDB saga"));
    }
    #[test]
    fn missing_media_is_distinct_from_failed_or_corrupt_catalogue_reads() {
        let db = db();
        assert_eq!(
            db.get("missing").unwrap_err(),
            "Vidéo introuvable dans le catalogue."
        );
        db.connection
            .execute("INSERT INTO media(id,data) VALUES('broken','not-json')", [])
            .unwrap();
        assert_eq!(
            db.get("broken").unwrap_err(),
            db_error(rusqlite::Error::InvalidQuery)
        );
        db.connection.execute_batch("DROP TABLE media;").unwrap();
        assert_eq!(
            db.get("broken").unwrap_err(),
            db_error(rusqlite::Error::InvalidQuery)
        );
    }
    #[test]
    fn batch_collection_add_is_atomic_bounded_idempotent_and_keeps_selection_order() {
        let mut db = db();
        let a = movie("Zulu.mkv");
        let b = movie("Alpha.mkv");
        let mut episode = movie("Episode.mkv");
        episode.category = "serie".into();
        for m in [&a, &b, &episode] {
            db.put(m).unwrap();
        }
        let id = db.create_collection("Saga", "saga").unwrap()[0].id.clone();
        for bad in [
            vec![],
            vec![a.id.clone(), "missing".into()],
            vec![a.id.clone(), episode.id.clone()],
            vec![a.id.clone(), a.id.clone()],
            vec![a.id.clone(); 1001],
        ] {
            assert!(db.add_collection_members(&id, &bad).is_err());
            assert!(db.collections().unwrap()[0].media_ids.is_empty());
        }
        db.add_collection_members(&id, &[a.id.clone(), b.id.clone()])
            .unwrap();
        assert_eq!(
            db.collections().unwrap()[0].media_ids,
            [a.id.clone(), b.id.clone()]
        );
        db.add_collection_members(&id, &[b.id.clone(), a.id.clone()])
            .unwrap();
        assert_eq!(
            db.collections().unwrap()[0].media_ids,
            [a.id.clone(), b.id.clone()]
        );
        assert_eq!(db.get(&a.id).unwrap(), a);
        assert_eq!(db.get(&b.id).unwrap(), b);
        assert!(
            db.add_collection_members("not-a-collection", &[a.id])
                .is_err()
        );
    }
    #[test]
    fn batch_collection_add_rolls_back_after_partial_sql_failure() {
        let mut db = db();
        let a = movie("A.mkv");
        let b = movie("B.mkv");
        db.put(&a).unwrap();
        db.put(&b).unwrap();
        let id = db.create_collection("Liste", "playlist").unwrap()[0]
            .id
            .clone();
        db.connection.execute_batch("CREATE TRIGGER fail_second BEFORE INSERT ON collection_members WHEN (SELECT COUNT(*) FROM collection_members)>0 BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
        assert!(
            db.add_collection_members(&id, &[a.id.clone(), b.id.clone()])
                .is_err()
        );
        assert!(db.collections().unwrap()[0].media_ids.is_empty());
        assert_eq!(db.get(&a.id).unwrap(), a);
    }
    #[test]
    fn playlist_order_survives_reopen_append_remove_readd_and_legacy_writers() {
        let folder = tempfile::tempdir().unwrap();
        let db_path = folder.path().join("order.sqlite");
        let a = movie("Zulu.mkv");
        let b = movie("Alpha.mkv");
        let c = movie("Middle.mkv");
        let id;
        {
            let mut db = Database::open(&db_path).unwrap();
            for m in [&a, &b, &c] {
                db.put(m).unwrap();
            }
            id = db.create_collection("Liste", "playlist").unwrap()[0]
                .id
                .clone();
            db.add_collection_members(&id, &[a.id.clone(), b.id.clone()])
                .unwrap();
            let expected = [a.id.clone(), b.id.clone()];
            db.reorder_playlist(&id, &expected, &[b.id.clone(), a.id.clone()])
                .unwrap();
        }
        let mut db = Database::open(&db_path).unwrap();
        assert_eq!(
            db.collections().unwrap()[0].media_ids,
            [b.id.clone(), a.id.clone()]
        );
        db.add_collection_members(&id, &[b.id.clone(), c.id.clone()])
            .unwrap();
        assert_eq!(
            db.collections().unwrap()[0].media_ids,
            [b.id.clone(), a.id.clone(), c.id.clone()]
        );
        // An older binary writes only memberships; the shared trigger removes stale positions.
        db.connection
            .execute(
                "DELETE FROM collection_members WHERE collection_id=?1 AND media_id=?2",
                params![id, b.id],
            )
            .unwrap();
        db.connection
            .execute(
                "INSERT INTO collection_members(collection_id,media_id) VALUES(?1,?2)",
                params![id, b.id],
            )
            .unwrap();
        assert_eq!(
            db.collections().unwrap()[0].media_ids,
            [a.id.clone(), c.id.clone(), b.id.clone()]
        );
        db.set_collection_member(&id, &a.id, false).unwrap();
        db.set_collection_member(&id, &a.id, true).unwrap();
        assert_eq!(
            db.collections().unwrap()[0].media_ids,
            [c.id.clone(), b.id.clone(), a.id.clone()]
        );
        for m in [&a, &b, &c] {
            assert_eq!(db.get(&m.id).unwrap(), *m);
        }
    }
    #[test]
    fn playlist_reorder_rejects_stale_or_invalid_permutations_and_rolls_back_sql() {
        let mut db = db();
        let a = movie("A.mkv");
        let b = movie("B.mkv");
        let c = movie("C.mkv");
        for m in [&a, &b, &c] {
            db.put(m).unwrap();
        }
        let id = db.create_collection("Liste", "playlist").unwrap()[0]
            .id
            .clone();
        let old = vec![a.id.clone(), b.id.clone()];
        db.add_collection_members(&id, &old).unwrap();
        for invalid in [
            vec![],
            vec![a.id.clone()],
            vec![a.id.clone(), a.id.clone()],
            vec![a.id.clone(), c.id.clone()],
        ] {
            assert!(db.reorder_playlist(&id, &old, &invalid).is_err());
            assert_eq!(db.collections().unwrap()[0].media_ids, old);
        }
        let new = vec![b.id.clone(), a.id.clone()];
        db.reorder_playlist(&id, &old, &new).unwrap();
        assert!(db.reorder_playlist(&id, &old, &old).is_err());
        db.connection.execute_batch("CREATE TRIGGER fail_order BEFORE INSERT ON collection_positions WHEN NEW.position=1 BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
        assert!(db.reorder_playlist(&id, &new, &old).is_err());
        assert_eq!(db.collections().unwrap()[0].media_ids, new);
        db.add_collection_members(&id, &[c.id]).unwrap();
        assert!(db.reorder_playlist(&id, &new, &old).is_err());
        let saga = db
            .create_collection("Saga", "saga")
            .unwrap()
            .into_iter()
            .find(|c| c.kind == "saga")
            .unwrap();
        assert!(db.reorder_playlist(&saga.id, &[], &[]).is_err());
        assert!(db.reorder_playlist("missing", &[], &[]).is_err());
    }
    #[test]
    fn old_database_preserves_insertion_order_and_forgetting_cleans_order_only() {
        let folder = tempfile::tempdir().unwrap();
        let db_path = folder.path().join("old.sqlite");
        let mut a = movie("Zulu.mkv");
        let b = movie("Alpha.mkv");
        a.available = false;
        let id;
        {
            let mut db = Database::open(&db_path).unwrap();
            db.put(&a).unwrap();
            db.put(&b).unwrap();
            id = db.create_collection("Liste", "playlist").unwrap()[0]
                .id
                .clone();
            db.add_collection_members(&id, &[a.id.clone(), b.id.clone()])
                .unwrap();
            db.connection
                .execute_batch(
                    "DROP TRIGGER remove_collection_position; DROP TABLE collection_positions;",
                )
                .unwrap();
        }
        let mut db = Database::open(&db_path).unwrap();
        let expected = [a.id.clone(), b.id.clone()];
        assert_eq!(db.collections().unwrap()[0].media_ids, expected);
        db.reorder_playlist(&id, &expected, &[b.id.clone(), a.id.clone()])
            .unwrap();
        db.remove_unavailable(&[a.id.clone()]).unwrap();
        assert_eq!(
            db.collections().unwrap()[0].media_ids,
            std::slice::from_ref(&b.id)
        );
        let count: i64 = db
            .connection
            .query_row("SELECT COUNT(*) FROM collection_positions", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count, 1);
        db.delete_collection(&id).unwrap();
        let count: i64 = db
            .connection
            .query_row("SELECT COUNT(*) FROM collection_positions", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count, 0);
        assert_eq!(db.get(&b.id).unwrap(), b);
    }
    #[test]
    fn saga_rename_keeps_identity_membership_and_legacy_import_durable() {
        let mut db = db();
        let mut film = movie("Alien.mkv");
        film.saga = Some("Alien".into());
        db.put(&film).unwrap();
        let id = db.collections().unwrap()[0].id.clone();
        let expected = db.get(&film.id).unwrap();
        let updated = db.rename_saga(&id, "  Univers Alien  ").unwrap();
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].id, id);
        assert_eq!(updated[0].name, "Univers Alien");
        assert_eq!(updated[0].media_ids, [film.id.clone()]);
        assert_eq!(db.get(&film.id).unwrap(), expected);
        assert_eq!(db.collections().unwrap().len(), 1);
        db.create_collection("Collision", "saga").unwrap();
        assert!(db.rename_saga(&id, "Collision").is_err());
        assert!(db.rename_saga(&id, " ").is_err());
        assert!(db.rename_saga(&id, &"x".repeat(201)).is_err());
        assert!(db.rename_saga("missing", "Nom").is_err());
        assert_eq!(
            db.collections()
                .unwrap()
                .iter()
                .find(|c| c.id == id)
                .unwrap()
                .name,
            "Univers Alien"
        );
        db.delete_collection(&id).unwrap();
        assert!(!db.collections().unwrap().iter().any(|c| c.name == "Alien"));
    }

    #[test]
    fn saga_images_are_independent_of_members_and_previous_selection_is_protected() {
        let mut db = db();
        let mut film = movie("Alien.mkv");
        film.poster = Some("/cache/film.jpg".into());
        db.put(&film).unwrap();
        let id = db.create_collection("Saga", "saga").unwrap()[0].id.clone();
        assert!(db.use_saga_member_poster(&id, &film.id).is_err());
        db.set_collection_member(&id, &film.id, true).unwrap();
        db.use_saga_member_poster(&id, &film.id).unwrap();
        assert_eq!(db.saga(&id).unwrap(), film.poster);
        let stale = None;
        assert!(
            db.set_saga_poster(&id, &stale, Some("/new.jpg".into()))
                .is_err()
        );
        let previous = db.saga(&id).unwrap();
        db.set_saga_poster(&id, &previous, Some("/imported.png".into()))
            .unwrap();
        assert_eq!(db.get(&film.id).unwrap(), film);
        db.set_collection_member(&id, &film.id, false).unwrap();
        assert_eq!(db.saga(&id).unwrap().as_deref(), Some("/imported.png"));
        assert!(db.use_saga_member_poster(&id, &film.id).is_err());
        let previous = db.saga(&id).unwrap();
        db.set_saga_poster(&id, &previous, None).unwrap();
        assert_eq!(db.saga(&id).unwrap(), None);
        db.set_saga_poster(&id, &None, Some("/pin.jpg".into()))
            .unwrap();
        db.delete_collection(&id).unwrap();
        assert!(
            db.set_saga_poster(&id, &Some("/pin.jpg".into()), Some("/late.jpg".into()))
                .is_err()
        );
        let count: i64 = db
            .connection
            .query_row("SELECT COUNT(*) FROM collection_artwork", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
        let playlist = db.create_collection("Liste", "playlist").unwrap()[0]
            .id
            .clone();
        assert!(db.rename_saga(&playlist, "Renamed").is_err());
        assert!(
            db.set_saga_poster(&playlist, &None, Some("/x.jpg".into()))
                .is_err()
        );
    }

    #[test]
    fn saga_artwork_failure_rolls_back_and_reopens_without_losing_catalogue() {
        let folder = tempfile::tempdir().unwrap();
        let db_path = folder.path().join("catalogue.sqlite");
        let id;
        let film = movie("Alien.mkv");
        {
            let mut db = Database::open(&db_path).unwrap();
            db.put(&film).unwrap();
            id = db.create_collection("Saga", "saga").unwrap()[0].id.clone();
            db.set_collection_member(&id, &film.id, true).unwrap();
            db.set_saga_poster(&id, &None, Some("/old.jpg".into()))
                .unwrap();
            db.connection.execute_batch("CREATE TRIGGER fail_image BEFORE UPDATE ON collection_artwork BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
            assert!(
                db.set_saga_poster(&id, &Some("/old.jpg".into()), Some("/new.jpg".into()))
                    .is_err()
            );
            assert_eq!(db.saga(&id).unwrap().as_deref(), Some("/old.jpg"));
        }
        let mut db = Database::open(&db_path).unwrap();
        assert_eq!(db.get(&film.id).unwrap(), film);
        let loaded = db.collections().unwrap();
        assert_eq!(loaded[0].id, id);
        assert_eq!(loaded[0].media_ids, [film.id]);
        assert_eq!(loaded[0].poster.as_deref(), Some("/old.jpg"));
        let legacy: Collection =
            serde_json::from_str(r#"{"id":"1","name":"Old","kind":"saga","media_ids":[]}"#)
                .unwrap();
        assert!(legacy.poster.is_none());
    }
    use crate::scanner::parse_file;
    #[test]
    fn existing_database_gains_optional_saga_artwork_without_rewriting_media() {
        let folder = tempfile::tempdir().unwrap();
        let db_path = folder.path().join("legacy.sqlite");
        let film = movie("Alien.mkv");
        let id;
        {
            let mut db = Database::open(&db_path).unwrap();
            db.put(&film).unwrap();
            id = db.create_collection("Legacy", "saga").unwrap()[0]
                .id
                .clone();
            db.set_collection_member(&id, &film.id, true).unwrap();
            // Equivalent to the prior schema: no artwork table, existing catalogue untouched.
            db.connection
                .execute_batch("DROP TABLE collection_artwork;")
                .unwrap();
        }
        let mut db = Database::open(&db_path).unwrap();
        assert_eq!(db.get(&film.id).unwrap(), film);
        let collections = db.collections().unwrap();
        assert_eq!(collections[0].id, id);
        assert_eq!(collections[0].media_ids, [film.id]);
        assert!(collections[0].poster.is_none());
    }
    fn movie(name: &str) -> Media {
        parse_file(Path::new("/nas"), &Path::new("/nas").join(name))
    }
    fn db() -> Database {
        Database::open(Path::new(":memory:")).unwrap()
    }
    #[test]
    fn source_root_keeps_unidentified_homonymous_series_apart() {
        let mut old = movie("Doctor Who (2005)/Season 01/Doctor.Who.S01E01.mkv");
        let mut new = movie("Doctor Who (2023)/Season 01/Doctor.Who.S01E01.mkv");
        for media in [&mut old, &mut new] {
            media.category = "serie".into();
            media.series = Some("Doctor Who".into());
        }
        assert_ne!(legacy_series_root(&old), legacy_series_root(&new));
        assert!(!same_series_group(&old, &new));
        let db = db();
        db.put(&old).unwrap();
        db.put(&new).unwrap();
        assert_eq!(db.series_members_for(&old).unwrap().len(), 1);
        assert_eq!(db.series_members_for(&new).unwrap().len(), 1);
    }
    #[test]
    fn source_group_normalizes_series_and_folder_spelling_without_merging_remakes() {
        let mut first = movie("Séries/Été Show/Season 01/E01.mkv");
        let mut variant = movie("series/ete   show/Season 01/E02.mkv");
        let mut remake = movie("series/ete show (2023)/Season 01/E01.mkv");
        for (media, name) in [
            (&mut first, "Été Show"),
            (&mut variant, "ete   show"),
            (&mut remake, "Été Show"),
        ] {
            media.category = "serie".into();
            media.series = Some(name.into());
        }
        assert_eq!(series_source_key(&first), series_source_key(&variant));
        assert_ne!(series_source_key(&first), series_source_key(&remake));
        assert!(same_series_group(&first, &variant));
        assert!(!same_series_group(&first, &remake));

        // A provider identity is stronger than any display spelling: two TMDB
        // ids remain distinct even if their folders happen to be the same.
        first.tmdb_id = Some(101);
        variant.tmdb_id = Some(202);
        assert!(!same_series_group(&first, &variant));
    }
    #[test]
    fn forget_unavailable_is_atomic_cleans_memberships_and_keeps_files_and_collections() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Alien.mkv");
        std::fs::write(&path, b"unchanged video fixture").unwrap();
        let mut missing = crate::scanner::parse_file(dir.path(), &path);
        missing.available = false;
        missing.saga = Some("Alien".into());
        missing.rating = Some(4);
        let available = movie("Arrival.mkv");
        let mut db = db();
        db.put(&missing).unwrap();
        db.put(&available).unwrap();
        let collections = db.create_collection("À voir", "playlist").unwrap();
        let list_id = &collections
            .iter()
            .find(|c| c.kind == "playlist")
            .unwrap()
            .id;
        db.set_collection_member(list_id, &missing.id, true)
            .unwrap();
        let before = db.list().unwrap();
        assert!(
            db.remove_unavailable(&[missing.id.clone(), available.id.clone()])
                .is_err()
        );
        assert_eq!(db.list().unwrap(), before);
        assert!(
            db.remove_unavailable(&[missing.id.clone(), "unknown".into()])
                .is_err()
        );
        assert!(
            db.remove_unavailable(&[missing.id.clone(), missing.id.clone()])
                .is_err()
        );
        assert!(db.remove_unavailable(&[]).is_err());
        let deleted = db
            .remove_unavailable(std::slice::from_ref(&missing.id))
            .unwrap();
        assert_eq!(deleted, vec![missing.id.clone()]);
        assert!(db.get(&missing.id).is_err());
        assert_eq!(db.get(&available.id).unwrap(), available);
        assert_eq!(std::fs::read(&path).unwrap(), b"unchanged video fixture");
        let collections = db.collections().unwrap();
        assert_eq!(collections.len(), 2);
        assert!(collections.iter().all(|c| c.media_ids.is_empty()));
        // Rediscovery is allowed, but forgotten user metadata is not secretly recreated.
        let rediscovered = crate::scanner::parse_file(dir.path(), &path);
        db.snapshot(vec![rediscovered.clone(), available]).unwrap();
        assert_eq!(db.get(&rediscovered.id).unwrap().rating, None);
    }
    #[test]
    fn unavailable_removal_rechecks_latest_scan_and_does_not_change_timestamp() {
        let mut db = db();
        let first = movie("Alien.mkv");
        let second = movie("Arrival.mkv");
        db.snapshot(vec![first.clone(), second.clone()]).unwrap();
        db.snapshot(vec![second.clone()]).unwrap();
        assert!(!db.get(&first.id).unwrap().available);
        db.snapshot(vec![first.clone(), second]).unwrap();
        let time = db.last_scan();
        assert!(
            db.remove_unavailable(std::slice::from_ref(&first.id))
                .is_err()
        );
        assert_eq!(db.last_scan(), time);
        assert!(db.get(&first.id).unwrap().available);
    }
    #[test]
    fn unavailable_batch_rolls_back_if_any_sql_write_fails() {
        let mut db = db();
        let mut first = movie("Alien.mkv");
        first.available = false;
        let mut second = movie("Arrival.mkv");
        second.available = false;
        db.put(&first).unwrap();
        db.put(&second).unwrap();
        db.connection.execute_batch("CREATE TRIGGER reject_delete BEFORE DELETE ON media WHEN OLD.id='Arrival.mkv' BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
        let before = db.list().unwrap();
        assert!(
            db.remove_unavailable(&[first.id.clone(), second.id.clone()])
                .is_err()
        );
        assert_eq!(db.list().unwrap(), before);
        db.connection
            .execute_batch("DROP TRIGGER reject_delete;")
            .unwrap();
        assert_eq!(
            db.remove_unavailable(&[first.id, second.id]).unwrap().len(),
            2
        );
        assert!(db.list().unwrap().is_empty());
    }
    #[test]
    fn unavailable_removal_waits_for_another_connection_and_rechecks_rediscovery() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("catalog.sqlite");
        let mut remover = Database::open(&path).unwrap();
        let mut missing = movie("Alien.mkv");
        missing.available = false;
        remover.put(&missing).unwrap();
        let mut writer = Database::open(&path).unwrap();
        let tx = writer
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .unwrap();
        let mut available = missing.clone();
        available.available = true;
        tx.execute(
            "UPDATE media SET data=?1 WHERE id=?2",
            params![serde_json::to_string(&available).unwrap(), available.id],
        )
        .unwrap();
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (result_tx, result_rx) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            started_tx.send(()).unwrap();
            result_tx
                .send(remover.remove_unavailable(&[missing.id]))
                .unwrap();
        });
        started_rx.recv().unwrap();
        assert!(matches!(
            result_rx.recv_timeout(std::time::Duration::from_millis(30)),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        ));
        tx.commit().unwrap();
        assert!(
            result_rx
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap()
                .is_err()
        );
        worker.join().unwrap();
        assert!(writer.get(&available.id).unwrap().available);
    }
    #[test]
    fn collections_migrate_once_allow_multiple_and_preserve_removals() {
        let mut db = db();
        let mut film = movie("Alien.1979.mkv");
        film.saga = Some("Alien".into());
        let episode = movie("Dark.S01E01.mkv");
        db.snapshot(vec![film.clone(), episode.clone()]).unwrap();
        let imported = db.collections().unwrap();
        let legacy_id = imported[0].id.clone();
        db.create_collection("Science-fiction", "saga").unwrap();
        let other = db
            .collections()
            .unwrap()
            .into_iter()
            .find(|c| c.name == "Science-fiction")
            .unwrap();
        db.set_collection_member(&other.id, &film.id, true).unwrap();
        assert_eq!(
            db.collections()
                .unwrap()
                .iter()
                .filter(|c| c.media_ids.contains(&film.id))
                .count(),
            2
        );
        db.set_collection_member(&legacy_id, &film.id, false)
            .unwrap();
        db.snapshot(vec![film.clone(), episode.clone()]).unwrap();
        assert!(
            db.collections()
                .unwrap()
                .iter()
                .find(|c| c.id == legacy_id)
                .unwrap()
                .media_ids
                .is_empty()
        );
        assert!(
            db.set_collection_member(&other.id, &episode.id, true)
                .is_err()
        );
        db.create_collection("À voir", "playlist").unwrap();
        let list = db
            .collections()
            .unwrap()
            .into_iter()
            .find(|c| c.kind == "playlist")
            .unwrap();
        db.set_collection_member(&list.id, &episode.id, true)
            .unwrap();
        db.set_collection_member(&list.id, &film.id, true).unwrap();
        db.set_collection_member(&list.id, &episode.id, false)
            .unwrap();
        assert_eq!(
            db.collections()
                .unwrap()
                .into_iter()
                .find(|c| c.id == list.id)
                .unwrap()
                .media_ids,
            vec![film.id]
        );
        assert!(db.create_collection("À voir", "playlist").is_err());
        assert!(db.create_collection("", "saga").is_err());
        assert!(db.create_collection("x", "unknown").is_err());
        assert!(db.set_collection_member("999999", "missing", true).is_err());
    }

    #[test]
    fn ratings_propagate_atomically_and_survive_scan_and_metadata() {
        let mut db = db();
        let first = movie("Dark.S01E01.mkv");
        let second = movie("Dark.S01E02.mkv");
        let next = movie("Dark.S02E01.mkv");
        let other = movie("Lost.S01E01.mkv");
        let film = movie("Alien.mkv");
        let all = vec![
            first.clone(),
            second.clone(),
            next.clone(),
            other.clone(),
            film.clone(),
        ];
        db.snapshot(all.clone()).unwrap();
        db.set_rating(&first.id, 5, "series").unwrap();
        db.set_rating(&first.id, 3, "season").unwrap();
        db.set_rating(&second.id, 1, "video").unwrap();
        assert_eq!(db.get(&first.id).unwrap().rating, Some(3));
        assert_eq!(db.get(&second.id).unwrap().rating, Some(1));
        assert_eq!(db.get(&next.id).unwrap().rating, Some(5));
        assert_eq!(db.get(&other.id).unwrap().rating, None);
        assert!(db.set_rating(&film.id, 5, "series").is_err());
        assert!(db.set_rating(&first.id, 6, "series").is_err());
        assert_eq!(db.get(&first.id).unwrap().rating, Some(3));
        db.snapshot(all).unwrap();
        let latest = db.get(&second.id).unwrap();
        assert_eq!(
            crate::model::merge_enrichment(&second, &latest, second.clone()).rating,
            Some(1)
        );
        db.set_rating(&second.id, 0, "video").unwrap();
        assert_eq!(db.get(&second.id).unwrap().rating, None);
    }

    #[test]
    fn collection_and_rating_data_reopen_without_losing_legacy_media() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("catalog.sqlite");
        let film = movie("Alien.mkv");
        {
            let mut db = Database::open(&file).unwrap();
            db.snapshot(vec![film.clone()]).unwrap();
            let lists = db.create_collection("Vide", "playlist").unwrap();
            assert!(lists[0].media_ids.is_empty());
            db.set_rating(&film.id, 4, "video").unwrap();
        }
        let mut db = Database::open(&file).unwrap();
        assert_eq!(db.get(&film.id).unwrap().rating, Some(4));
        assert_eq!(db.collections().unwrap()[0].name, "Vide");
    }
    #[test]
    fn collection_deletion_never_deletes_media_or_resurrects_legacy_saga() {
        let mut db = db();
        let mut film = movie("Alien.mkv");
        film.saga = Some("Alien".into());
        db.snapshot(vec![film.clone()]).unwrap();
        let id = db.collections().unwrap()[0].id.clone();
        db.delete_collection(&id).unwrap();
        assert!(db.collections().unwrap().is_empty());
        assert_eq!(db.get(&film.id).unwrap(), film);
        let lists = db.create_collection("Nouvelle", "playlist").unwrap();
        assert_ne!(lists[0].id, id);
        db.set_collection_member(&lists[0].id, &film.id, true)
            .unwrap();
        db.delete_collection(&lists[0].id).unwrap();
        assert!(db.collections().unwrap().is_empty());
        assert!(db.delete_collection(&id).is_err());
        assert_eq!(db.get(&film.id).unwrap(), film);
    }
    #[test]
    fn refresh_keeps_watched_and_manual_metadata() {
        let mut db = db();
        let a = movie("Alien.1979.mkv");
        db.snapshot(vec![a.clone()]).unwrap();
        db.patch(
            &a.id,
            &MediaPatch {
                watched: Some(true),
                saga: Some("Alien".into()),
                title: Some("Alien : le huitième passager".into()),
                ..Default::default()
            },
        )
        .unwrap();
        db.snapshot(vec![a.clone()]).unwrap();
        let result = db.get(&a.id).unwrap();
        assert!(result.watched);
        assert_eq!(result.saga.as_deref(), Some("Alien"));
        assert!(result.title.contains("passager"));
    }
    #[test]
    fn empty_scan_preserves_last_success() {
        let mut db = db();
        let a = movie("Alien.mkv");
        let time = db.snapshot(vec![a.clone()]).unwrap();
        assert!(db.snapshot(vec![]).is_err());
        assert_eq!(db.last_scan(), Some(time));
        assert_eq!(db.list().unwrap(), vec![a]);
    }
    #[test]
    fn missing_files_preserve_history() {
        let mut db = db();
        let a = movie("Alien.mkv");
        let b = movie("Arrival.mkv");
        db.snapshot(vec![a.clone(), b.clone()]).unwrap();
        db.snapshot(vec![b]).unwrap();
        assert!(!db.get(&a.id).unwrap().available);
    }
    #[test]
    fn invalid_assistant_change_is_atomic() {
        let mut db = db();
        let a = movie("Alien.mkv");
        db.snapshot(vec![a.clone()]).unwrap();
        let result = db.apply_changes(&[
            (
                a.id.clone(),
                MediaPatch {
                    watched: Some(true),
                    ..Default::default()
                },
            ),
            ("unknown".into(), MediaPatch::default()),
        ]);
        assert!(result.is_err());
        assert!(!db.get(&a.id).unwrap().watched);
    }
    #[test]
    fn settings_do_not_store_secrets() {
        let db = db();
        let s = Settings {
            tmdb_token: "secret".into(),
            ..Default::default()
        };
        db.save_settings(&s).unwrap();
        assert!(db.settings().unwrap().tmdb_token.is_empty());
    }
    #[test]
    fn invalid_patch_does_not_persist() {
        let db = db();
        let a = movie("Alien.mkv");
        db.put(&a).unwrap();
        assert!(
            db.patch(
                &a.id,
                &MediaPatch {
                    category: Some("bad".into()),
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert_eq!(db.get(&a.id).unwrap(), a);
    }
    #[test]
    fn recommendations_favour_saga_and_genres() {
        let mut a = movie("Alien.mkv");
        a.saga = Some("Alien".into());
        a.genres = vec!["Science-fiction".into()];
        let mut b = movie("Aliens.mkv");
        b.saga = a.saga.clone();
        let mut c = movie("Arrival.mkv");
        c.genres = a.genres.clone();
        assert_eq!(similar(&a, vec![a.clone(), c, b.clone()])[0].id, b.id);
    }
    #[test]
    fn explicit_null_clears_episode_but_omission_preserves_it() {
        let db = db();
        let a = crate::scanner::parse_file(Path::new("/nas"), Path::new("/nas/Dark.S01E02.mkv"));
        db.put(&a).unwrap();
        db.patch(
            &a.id,
            &serde_json::from_str::<MediaPatch>(r#"{"title":"Changed"}"#).unwrap(),
        )
        .unwrap();
        assert_eq!(db.get(&a.id).unwrap().episode, Some(2));
        db.patch(
            &a.id,
            &serde_json::from_str::<MediaPatch>(r#"{"season":null,"episode":null}"#).unwrap(),
        )
        .unwrap();
        let m = db.get(&a.id).unwrap();
        assert_eq!(m.season, None);
        assert_eq!(m.episode, None);
    }
    #[test]
    fn scan_budget_error_keeps_database_and_timestamp() {
        let mut db = db();
        let a = movie("Alien.mkv");
        db.snapshot(vec![a]).unwrap();
        let before = db.list().unwrap();
        let time = db.last_scan();
        let folder = tempfile::tempdir().unwrap();
        std::fs::write(folder.path().join("Alien.mkv"), b"").unwrap();
        assert!(crate::scanner::discover_with_budget(folder.path(), 5).is_err());
        assert_eq!(db.list().unwrap(), before);
        assert_eq!(db.last_scan(), time);
    }
    #[test]
    fn tmdb_merge_preserves_edits_and_clears_in_flight() {
        let original = movie("Alien.mkv");
        let mut latest = original.clone();
        latest.title = "Mon titre".into();
        latest.category = "documentaire".into();
        latest.year = Some(1990);
        latest.genres = vec!["Mon genre".into()];
        latest.series = Some("Ma série".into());
        latest.season = Some(2);
        latest.episode = Some(3);
        latest.overview = "Mon résumé".into();
        let mut downloaded = original.clone();
        downloaded.title = "TMDB".into();
        downloaded.category = "film".into();
        downloaded.saga = Some("Saga TMDB".into());
        let merged = crate::model::merge_enrichment(&original, &latest, downloaded);
        assert_eq!(merged.title, latest.title);
        assert_eq!(merged.category, latest.category);
        assert_eq!(merged.year, latest.year);
        assert_eq!(merged.genres, latest.genres);
        assert_eq!(merged.series, latest.series);
        assert_eq!(merged.season, latest.season);
        assert_eq!(merged.episode, latest.episode);
        assert_eq!(merged.overview, latest.overview);
        let mut original = original;
        original.saga = Some("Personnelle".into());
        original.season = Some(1);
        let mut latest = original.clone();
        latest.saga = None;
        latest.season = None;
        let mut downloaded = original.clone();
        downloaded.saga = Some("TMDB".into());
        let merged = crate::model::merge_enrichment(&original, &latest, downloaded);
        assert_eq!(merged.saga, None);
        assert_eq!(merged.season, None);
    }
    #[test]
    fn old_catalogue_rows_load_with_presentation_defaults() {
        let db = db();
        let a = movie("Alien.mkv");
        let mut legacy = serde_json::to_value(&a).unwrap();
        for field in [
            "runtime_minutes",
            "cast",
            "directors",
            "poster_custom",
            "series_overview",
            "series_metadata",
            "series_manual",
            "rating",
        ] {
            legacy.as_object_mut().unwrap().remove(field);
        }
        db.connection
            .execute(
                "INSERT INTO media(id,data) VALUES(?1,?2)",
                params![a.id, legacy.to_string()],
            )
            .unwrap();
        let loaded = db.get(&a.id).unwrap();
        assert_eq!(loaded.runtime_minutes, None);
        assert!(loaded.cast.is_empty());
        assert!(loaded.directors.is_empty());
        assert!(!loaded.poster_custom);
        assert!(loaded.series_overview.is_empty());
        assert!(loaded.series_metadata.is_none());
        assert!(!loaded.series_manual);
        assert_eq!(loaded.rating, None);
    }
    #[test]
    fn editable_presentation_persists_and_refresh_keeps_it() {
        let mut db = db();
        let a = movie("Alien.mkv");
        db.put(&a).unwrap();
        let patch: MediaPatch = serde_json::from_value(serde_json::json!({"runtime_minutes":117,"cast":[" Sigourney Weaver "],"directors":["Ridley Scott"]})).unwrap();
        db.patch(&a.id, &patch).unwrap();
        db.snapshot(vec![a.clone()]).unwrap();
        let updated = db.get(&a.id).unwrap();
        assert_eq!(updated.runtime_minutes, Some(117));
        assert_eq!(updated.cast, ["Sigourney Weaver"]);
        assert_eq!(updated.directors, ["Ridley Scott"]);
        let invalid: MediaPatch =
            serde_json::from_value(serde_json::json!({"runtime_minutes":-1,"cast":["changed"]}))
                .unwrap();
        assert!(db.patch(&a.id, &invalid).is_err());
        assert_eq!(db.get(&a.id).unwrap(), updated);
        let clear: MediaPatch = serde_json::from_value(
            serde_json::json!({"runtime_minutes":0,"cast":[],"directors":[]}),
        )
        .unwrap();
        let cleared = db.patch(&a.id, &clear).unwrap();
        assert!(cleared.runtime_minutes.is_none());
        assert!(cleared.cast.is_empty());
        assert!(cleared.directors.is_empty());
    }
    #[test]
    fn enrichment_preserves_custom_poster_and_concurrent_presentation_edits() {
        let mut original = movie("Alien.mkv");
        original.poster_custom = true;
        original.poster = Some("/cache/custom.jpg".into());
        let mut latest = original.clone();
        latest.runtime_minutes = Some(123);
        latest.cast = vec!["Manual actor".into()];
        latest.directors = vec!["Manual director".into()];
        let mut downloaded = original.clone();
        downloaded.poster = Some("/cache/tmdb.jpg".into());
        downloaded.poster_custom = false;
        downloaded.runtime_minutes = Some(117);
        let merged = crate::model::merge_enrichment(&original, &latest, downloaded);
        assert_eq!(merged.poster, latest.poster);
        assert!(merged.poster_custom);
        assert_eq!(merged.runtime_minutes, latest.runtime_minutes);
        assert_eq!(merged.cast, latest.cast);
        assert_eq!(merged.directors, latest.directors);
    }
    #[test]
    fn changing_tmdb_namespace_requires_a_new_association() {
        let db = db();
        let mut a = movie("Alien.mkv");
        a.tmdb_id = Some(348);
        a.poster = Some("/cache/movie.jpg".into());
        db.put(&a).unwrap();
        let updated = db
            .patch(
                &a.id,
                &MediaPatch {
                    category: Some("serie".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(updated.tmdb_id.is_none());
        assert!(updated.poster.is_none());
    }
    #[test]
    fn poster_restore_is_atomic_and_keeps_concurrent_import() {
        let db = db();
        let mut original = movie("Alien.mkv");
        original.poster = Some("/cache/custom.png".into());
        original.poster_custom = true;
        db.put(&original).unwrap();
        assert!(db.restore_poster(&original, None).is_err());
        assert_eq!(db.get(&original.id).unwrap(), original);
        let restored = db
            .restore_poster(&original, Some("/cache/movie.jpg".into()))
            .unwrap();
        assert_eq!(restored.poster.as_deref(), Some("/cache/movie.jpg"));
        assert!(!restored.poster_custom);
        let mut imported = original.clone();
        imported.poster = Some("/cache/new.png".into());
        db.put(&imported).unwrap();
        assert!(
            db.restore_poster(&original, Some("/cache/movie.jpg".into()))
                .is_err()
        );
        assert_eq!(db.get(&original.id).unwrap(), imported);
        let mut reassociated = original.clone();
        reassociated.tmdb_id = Some(999);
        db.put(&reassociated).unwrap();
        assert!(
            db.restore_poster(&original, Some("/cache/movie.jpg".into()))
                .is_err()
        );
        assert_eq!(db.get(&original.id).unwrap(), reassociated);
    }
    #[test]
    fn series_synopsis_updates_all_members_but_not_episode_synopsis() {
        let mut db = db();
        let mut a = movie("Dark.S01E01.mkv");
        a.overview = "First episode".into();
        let mut b = movie("Dark.S02E01.mkv");
        b.overview = "Second episode".into();
        let other = movie("Alien.mkv");
        db.snapshot(vec![a.clone(), b.clone(), other.clone()])
            .unwrap();
        let result = db.set_series_overview("Dark", "Whole series").unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(db.get(&a.id).unwrap().series_overview, "Whole series");
        assert_eq!(db.get(&b.id).unwrap().series_overview, "Whole series");
        assert_eq!(db.get(&a.id).unwrap().overview, "First episode");
        assert!(db.get(&other.id).unwrap().series_overview.is_empty());
        let before = db.list().unwrap();
        assert!(db.set_series_overview("Dark", &"x".repeat(20001)).is_err());
        assert_eq!(db.list().unwrap(), before);
    }

    fn series_patch(name: &str) -> SeriesPatch {
        SeriesPatch {
            name: name.into(),
            year: 2017,
            genres: vec![" Mystère ".into()],
            overview: "Synopsis corrigé".into(),
            cast: vec!["Actrice série".into()],
            directors: vec!["Réalisatrice série".into()],
        }
    }

    #[test]
    fn series_corrections_are_shared_without_overwriting_episode_data() {
        let mut db = db();
        let mut a = movie("Dark.S01E01.mkv");
        a.overview = "Résumé épisode".into();
        a.cast = vec!["Acteur épisode".into()];
        a.runtime_minutes = Some(42);
        a.rating = Some(4);
        a.watched = true;
        let b = movie("Dark.S02E01.mkv");
        let other = movie("Alien.mkv");
        db.snapshot(vec![a.clone(), b.clone(), other.clone()])
            .unwrap();
        let playlist = db.create_collection("Episodes", "playlist").unwrap()[0]
            .id
            .clone();
        db.set_collection_member(&playlist, &a.id, true).unwrap();
        let changed = db
            .update_series("Dark", &series_patch("Dark corrigée"))
            .unwrap();
        assert_eq!(changed.len(), 2);
        for original in [&a, &b] {
            let actual = db.get(&original.id).unwrap();
            let mut expected = original.clone();
            expected.series = Some("Dark corrigée".into());
            expected.series_manual = true;
            expected.series_overview = "Synopsis corrigé".into();
            expected.series_metadata =
                Some(series_patch("Dark corrigée").validate(original).unwrap());
            assert_eq!(actual, expected);
        }
        assert_eq!(db.get(&other.id).unwrap(), other);
        assert!(db.collections().unwrap()[0].media_ids.contains(&a.id));
        db.patch(
            &a.id,
            &MediaPatch {
                overview: Some("Correction épisode".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(db.get(&a.id).unwrap().overview, "Correction épisode");
        assert_eq!(db.get(&b.id).unwrap().overview, b.overview);
        let latest = db.get(&a.id).unwrap();
        let merged = crate::model::merge_enrichment(&latest, &latest, a.clone());
        assert_eq!(merged.series_metadata, latest.series_metadata);
        assert_eq!(merged.series, latest.series);
        assert_eq!(merged.series_overview, latest.series_overview);
        db.snapshot(vec![a, b, other]).unwrap();
        assert_eq!(
            db.get(&latest.id).unwrap().series_metadata,
            latest.series_metadata
        );
    }

    #[test]
    fn series_corrections_reject_invalid_and_collision_and_allow_clearing() {
        let mut db = db();
        db.snapshot(vec![movie("Dark.S01E01.mkv"), movie("Other.S01E01.mkv")])
            .unwrap();
        let before = db.list().unwrap();
        assert!(db.update_series("Dark", &series_patch("Other")).is_err());
        assert!(db.update_series("Missing", &series_patch("New")).is_err());
        for invalid in ["", &"x".repeat(501)] {
            assert!(db.update_series("Dark", &series_patch(invalid)).is_err());
        }
        let mut patch = series_patch("Dark");
        patch.year = 1;
        assert!(db.update_series("Dark", &patch).is_err());
        assert_eq!(db.list().unwrap(), before);
        assert!(serde_json::from_value::<SeriesPatch>(serde_json::json!({"name":"Dark", "year":0,"genres":[],"overview":"", "cast":[], "directors":[],"path":"/evil"})).is_err());
        patch.year = 0;
        patch.genres.clear();
        patch.overview.clear();
        patch.cast.clear();
        patch.directors.clear();
        let changed = db.update_series("Dark", &patch).unwrap();
        let metadata = changed[0].series_metadata.as_ref().unwrap();
        assert!(metadata.year.is_none());
        assert!(metadata.genres.is_empty());
        assert!(metadata.overview.is_empty());
        let moved = db
            .patch(
                &changed[0].id,
                &MediaPatch {
                    series: Some("Other".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(moved.series_metadata.is_none());
        assert!(moved.series_overview.is_empty());
    }

    #[test]
    fn series_update_rolls_back_on_write_error() {
        let mut db = db();
        db.snapshot(vec![movie("Dark.S01E01.mkv"), movie("Dark.S02E01.mkv")])
            .unwrap();
        let before = db.list().unwrap();
        db.connection.execute_batch("CREATE TRIGGER reject_series_update BEFORE UPDATE ON media WHEN OLD.id LIKE '%S02%' BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
        assert!(
            db.update_series("Dark", &series_patch("Corrected"))
                .is_err()
        );
        assert_eq!(db.list().unwrap(), before);
    }

    fn provider() -> SeriesProvider {
        SeriesProvider {
            name: "Série TMDB".into(),
            metadata: series_patch("Série TMDB")
                .validate(&movie("Dark.S01E01.mkv"))
                .unwrap(),
            tmdb_id: 42,
            poster: Some("/cache/tv-42.jpg".into()),
        }
    }

    #[test]
    fn series_provider_updates_all_seasons_and_preserves_episode_metadata_and_memberships() {
        let mut db = db();
        let mut a = movie("Dark.S01E01.mkv");
        a.overview = "Episode summary".into();
        a.runtime_minutes = Some(42);
        a.rating = Some(5);
        a.watched = true;
        a.cast = vec!["Episode actor".into()];
        a.directors = vec!["Episode director".into()];
        a.poster = Some("/custom.jpg".into());
        a.poster_custom = true;
        let b = movie("Dark.S02E01.mkv");
        let other = movie("Alien.mkv");
        db.snapshot(vec![a.clone(), b.clone(), other.clone()])
            .unwrap();
        let playlist = db.create_collection("Episodes", "playlist").unwrap()[0]
            .id
            .clone();
        db.set_collection_member(&playlist, &a.id, true).unwrap();
        let originals = db.series_members("Dark").unwrap();
        let p = provider();
        let changed = db
            .apply_series_provider("Dark", &originals, &p, true)
            .unwrap();
        assert_eq!(changed.len(), 2);
        for original in [&a, &b] {
            let mut expected = original.clone();
            expected.series = Some(p.name.clone());
            expected.series_manual = false;
            expected.series_metadata = Some(p.metadata.clone());
            expected.series_overview = p.metadata.overview.clone();
            expected.tmdb_id = Some(42);
            if !expected.poster_custom {
                expected.poster = p.poster.clone();
            }
            assert_eq!(db.get(&original.id).unwrap(), expected);
        }
        assert_eq!(db.get(&other.id).unwrap(), other);
        assert!(db.collections().unwrap()[0].media_ids.contains(&a.id));
    }

    #[test]
    fn series_provider_refresh_keeps_manual_name_and_poster_on_missing_image() {
        let mut db = db();
        let mut a = movie("Dark.S01E01.mkv");
        a.tmdb_id = Some(42);
        a.poster = Some("/existing.jpg".into());
        db.snapshot(vec![a]).unwrap();
        db.update_series("Dark", &series_patch("Nom corrigé"))
            .unwrap();
        let originals = db.series_members("Nom corrigé").unwrap();
        let mut p = provider();
        p.poster = None;
        let changed = db
            .apply_series_provider("Nom corrigé", &originals, &p, false)
            .unwrap();
        assert_eq!(changed[0].series.as_deref(), Some("Nom corrigé"));
        assert!(changed[0].series_manual);
        assert_eq!(changed[0].poster.as_deref(), Some("/existing.jpg"));
    }

    #[test]
    fn series_provider_rejects_stale_group_but_preserves_concurrent_episode_edits() {
        let mut db = db();
        let a = movie("Dark.S01E01.mkv");
        let b = movie("Dark.S02E01.mkv");
        db.snapshot(vec![a.clone(), b.clone()]).unwrap();
        let originals = db.series_members("Dark").unwrap();
        db.patch(
            &a.id,
            &MediaPatch {
                title: Some("Titre épisode corrigé".into()),
                watched: Some(true),
                ..Default::default()
            },
        )
        .unwrap();
        let changed = db
            .apply_series_provider("Dark", &originals, &provider(), true)
            .unwrap();
        assert_eq!(
            changed.iter().find(|m| m.id == a.id).unwrap().title,
            "Titre épisode corrigé"
        );
        assert!(changed.iter().find(|m| m.id == a.id).unwrap().watched);
        let originals = db.series_members("Série TMDB").unwrap();
        db.update_series("Série TMDB", &series_patch("Nouvelle correction"))
            .unwrap();
        let before = db.list().unwrap();
        assert!(
            db.apply_series_provider("Série TMDB", &originals, &provider(), true)
                .is_err()
        );
        assert_eq!(db.list().unwrap(), before);
        let originals = db.series_members("Nouvelle correction").unwrap();
        let mut c = movie("Nouvelle correction.S03E01.mkv");
        c.tmdb_id = originals[0].tmdb_id;
        db.put(&c).unwrap();
        let before = db.list().unwrap();
        assert!(
            db.apply_series_provider("Nouvelle correction", &originals, &provider(), true)
                .is_err()
        );
        assert_eq!(db.list().unwrap(), before);
    }

    #[test]
    fn series_provider_allows_title_collision_but_rolls_back_sql_failure() {
        let mut db = db();
        let a = movie("Dark.S01E01.mkv");
        let b = movie("Dark.S02E01.mkv");
        let other = movie("Other.S01E01.mkv");
        db.snapshot(vec![a, b, other]).unwrap();
        let originals = db.series_members("Dark").unwrap();
        let mut p = provider();
        p.name = "Other".into();
        let changed = db
            .apply_series_provider("Dark", &originals, &p, true)
            .unwrap();
        assert_eq!(changed.len(), 2);
        assert!(
            changed
                .iter()
                .all(|item| item.series.as_deref() == Some("Other")
                    && item.tmdb_id == Some(p.tmdb_id))
        );
        let untouched = db
            .list()
            .unwrap()
            .into_iter()
            .find(|item| !originals.iter().any(|original| original.id == item.id))
            .unwrap();
        assert_eq!(untouched.tmdb_id, None);
        assert_eq!(db.series_members_for(&untouched).unwrap().len(), 1);
        let originals = changed;
        let before = db.list().unwrap();
        p = provider();
        p.tmdb_id = -1;
        assert!(
            db.apply_series_provider("Dark", &originals, &p, true)
                .is_err()
        );
        assert_eq!(db.list().unwrap(), before);
        db.connection.execute_batch("CREATE TRIGGER reject_series_provider BEFORE UPDATE ON media WHEN OLD.id LIKE '%S02%' BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
        assert!(
            db.apply_series_provider("Dark", &originals, &provider(), true)
                .is_err()
        );
        assert_eq!(db.list().unwrap(), before);
    }

    #[test]
    fn episode_reassociation_inherits_target_metadata_and_survives_provider_refresh() {
        let mut db = db();
        let source = movie("Dark.S01E01.mkv");
        let target = movie("Other.S02E01.mkv");
        db.snapshot(vec![source.clone(), target.clone()]).unwrap();
        db.update_series("Other", &series_patch("Other corrigée"))
            .unwrap();
        let moved = db
            .patch(
                &source.id,
                &MediaPatch {
                    series: Some("Other corrigée".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(
            moved.series_metadata,
            db.get(&target.id).unwrap().series_metadata
        );
        assert!(moved.series_manual);
        let merged = crate::model::merge_enrichment(&moved, &moved, source.clone());
        assert_eq!(merged.series, moved.series);
        assert_eq!(merged.series_metadata, moved.series_metadata);
        let ungrouped = db
            .patch(
                &source.id,
                &MediaPatch {
                    series: Some("New group".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(ungrouped.series_metadata.is_none());
        assert!(ungrouped.series_manual);
        let merged = crate::model::merge_enrichment(&ungrouped, &ungrouped, source);
        assert_eq!(merged.series, ungrouped.series);
        db.apply_changes(&[(
            ungrouped.id.clone(),
            MediaPatch {
                series: Some("Other corrigée".into()),
                ..Default::default()
            },
        )])
        .unwrap();
        assert_eq!(
            db.get(&ungrouped.id).unwrap().series_metadata,
            db.get(&target.id).unwrap().series_metadata
        );
    }

    #[test]
    fn explicit_provider_selection_replaces_old_group_but_preserves_in_flight_corrections() {
        let mut original = movie("Dark.S01E01.mkv");
        original.series_manual = true;
        original.series_metadata = Some(series_patch("Dark").validate(&original).unwrap());
        original.tmdb_id = Some(1);
        let mut downloaded = original.clone();
        downloaded.series = Some("Other".into());
        downloaded.tmdb_id = Some(2);
        downloaded.series_overview = "Nouvelle série fournisseur".into();
        let refreshed = crate::model::merge_enrichment(&original, &original, downloaded.clone());
        assert_eq!(refreshed.series, original.series);
        let selected = crate::model::merge_association(&original, &original, downloaded.clone());
        assert_eq!(selected.series.as_deref(), Some("Other"));
        assert!(selected.series_metadata.is_none());
        assert!(!selected.series_manual);
        assert_eq!(selected.series_overview, "Nouvelle série fournisseur");
        let mut latest = original.clone();
        latest.series = Some("Correction pendant requête".into());
        let protected = crate::model::merge_association(&original, &latest, downloaded.clone());
        assert_eq!(protected.series, latest.series);
        assert_eq!(protected.series_metadata, latest.series_metadata);
        let mut latest = original.clone();
        latest.series_metadata.as_mut().unwrap().overview =
            "Correction synopsis concurrente".into();
        let protected = crate::model::merge_association(&original, &latest, downloaded.clone());
        assert_eq!(protected.series, latest.series);
        assert_eq!(protected.series_metadata, latest.series_metadata);
        let mut latest = original.clone();
        latest.series_overview = "Synopsis legacy concurrent".into();
        let protected = crate::model::merge_association(&original, &latest, downloaded);
        assert_eq!(protected.series_overview, latest.series_overview);
        assert_eq!(protected.series, latest.series);
    }
}
