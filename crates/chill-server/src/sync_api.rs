//! Additive 2026-10-14 catalogue changes API.  Legacy `/snapshot` remains
//! available for older desktop builds.
use crate::api::{ApiError, AppState};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use chill_domain::{
    SyncMedia, SyncSnapshot,
    catalog_delta::{self, CatalogChanges},
    store::{PlaylistView, SagaView},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};
use uuid::Uuid;

/// Deliberately outside SQLite and outside operational backup manifests.
/// Ordinary restarts reuse this private marker. The supported offline restore
/// publishes a fresh data tree without it, invalidating even an ABA epoch.
pub(crate) fn database_generation(database: &std::path::Path) -> Result<String, String> {
    use std::{
        fs::OpenOptions,
        io::{Read, Write},
        os::unix::fs::{MetadataExt, OpenOptionsExt},
    };
    let path = database.with_extension("sync-generation");
    let mut file = match OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(&path)
    {
        Ok(mut file) => {
            let generation = Uuid::new_v4().to_string();
            file.write_all(generation.as_bytes())
                .and_then(|()| file.sync_all())
                .map_err(|_| "Génération du catalogue inaccessible.".to_owned())?;
            return Ok(generation);
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&path)
            .map_err(|_| "Génération du catalogue inaccessible.".to_owned())?,
        Err(_) => return Err("Génération du catalogue inaccessible.".into()),
    };
    let metadata = file
        .metadata()
        .map_err(|_| "Génération du catalogue inaccessible.".to_owned())?;
    if !metadata.is_file()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o177 != 0
        || metadata.len() != 36
    {
        return Err("Génération du catalogue non sûre.".into());
    }
    let mut value = String::new();
    file.read_to_string(&mut value)
        .map_err(|_| "Génération du catalogue invalide.".to_owned())?;
    Uuid::parse_str(&value).map_err(|_| "Génération du catalogue invalide.".to_owned())?;
    Ok(value)
}

const TTL: Duration = Duration::from_secs(120);
const MAX_SNAPSHOTS: usize = 3;
#[derive(Clone)]
struct Baseline {
    server: String,
    profile: String,
    revision: String,
    created: Instant,
    /// The sole immutable image retained for an active paginated transfer.
    snapshot: Arc<SyncSnapshot>,
    serialized_bytes: usize,
}
#[derive(Default)]
struct Cache {
    values: HashMap<String, Baseline>,
}
static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
fn cache() -> &'static Mutex<Cache> {
    CACHE.get_or_init(|| Mutex::new(Cache::default()))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    known_revision: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Page {
    offset: Option<usize>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/profiles/{profile}/catalog/changes", post(changes))
        .route("/profiles/{profile}/catalog/changes/{snapshot}", get(page))
}
type SnapshotDocument = (
    Vec<SagaView>,
    Vec<PlaylistView>,
    std::collections::BTreeMap<String, u64>,
    std::collections::BTreeMap<String, chill_catalog::model::SeasonProvider>,
);
fn document(snapshot: &SyncSnapshot) -> SnapshotDocument {
    (
        snapshot.sagas.clone(),
        snapshot.playlists.clone(),
        snapshot.series_revisions.clone(),
        snapshot.season_metadata.clone(),
    )
}
fn changed(
    old: &catalog_delta::SignatureCheckpoint,
    now: &SyncSnapshot,
) -> Result<(Vec<SyncMedia>, Vec<String>), ApiError> {
    let next = now
        .items
        .iter()
        .map(|m| (m.media.id.as_str(), m))
        .collect::<HashMap<_, _>>();
    let upserts = now
        .items
        .iter()
        .filter(|m| {
            catalog_delta::item_signature(m).ok().as_deref()
                != old.items.get(&m.media.id).map(String::as_str)
        })
        .cloned()
        .collect();
    let deleted_ids = old
        .items
        .keys()
        .filter(|id| !next.contains_key(id.as_str()))
        .cloned()
        .collect();
    Ok((upserts, deleted_ids))
}
async fn context(
    state: &AppState,
    profile: String,
) -> Result<(String, String, SyncSnapshot), ApiError> {
    let server = state
        .certificate
        .lock()
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "SYNC_BUSY"))?
        .server_id
        .clone();
    let snapshot_profile = profile.clone();
    let (epoch, snapshot) = state
        .database
        .call(move |s| {
            let snapshot = s.sync_snapshot(&snapshot_profile)?;
            catalog_delta::bounded(&snapshot).map_err(str::to_owned)?;
            Ok((snapshot.captured_revision, snapshot))
        })
        .await?;
    let revision = catalog_delta::revision(
        &format!("{server}:{}", state.database.sync_generation),
        &profile,
        epoch,
    );
    Ok((server, revision, snapshot))
}
async fn current_revision(state: &AppState, profile: &str) -> Result<(String, String), ApiError> {
    let server = state
        .certificate
        .lock()
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "SYNC_BUSY"))?
        .server_id
        .clone();
    let profile = profile.to_owned();
    let profile_for_store = profile.clone();
    let epoch = state
        .database
        .call(move |store| store.sync_revision(&profile_for_store))
        .await?;
    Ok((
        server.clone(),
        catalog_delta::revision(
            &format!("{server}:{}", state.database.sync_generation),
            &profile,
            epoch,
        ),
    ))
}
async fn changes(
    State(state): State<AppState>,
    Path(profile): Path<String>,
    Json(request): Json<Request>,
) -> Result<Json<Value>, ApiError> {
    // The common case reads just durable epochs. Do not construct or serialize
    // the catalogue when the client is already current.
    let (_server, known_current) = current_revision(&state, &profile).await?;
    if request.known_revision.as_deref() == Some(&known_current) {
        return Ok(Json(json!({"revision":known_current,"unchanged":true})));
    }
    let durable_old = match request.known_revision.as_deref() {
        Some(known) if known.len() <= 255 => {
            let profile = profile.clone();
            let known = known.to_owned();
            state
                .database
                .call(move |store| store.catalog_delta_checkpoint(&profile, &known))
                .await?
        }
        _ => None,
    };
    let (server, revision, snapshot) = context(&state, profile.clone()).await?;
    let old = durable_old;
    let (items, deleted_ids, reset) = if let Some(old) = old {
        let (a, b) = changed(&old, &snapshot)?;
        (a, b, false)
    } else {
        (snapshot.items.clone(), Vec::new(), true)
    };
    let id = Uuid::new_v4().to_string();
    let total = items.len();
    let first = items[..total.min(catalog_delta::DELTA_PAGE)].to_vec();
    let (sagas, playlists, series_revisions, season_metadata) = document(&snapshot);
    let stored = Arc::new(SyncSnapshot {
        items,
        sagas: sagas.clone(),
        playlists: playlists.clone(),
        series_revisions: series_revisions.clone(),
        season_metadata: season_metadata.clone(),
        serialized_bytes: snapshot.serialized_bytes,
        captured_revision: snapshot.captured_revision,
    });
    let serialized_bytes = serde_json::to_vec(&*stored)
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "SYNC_INVALID"))?
        .len();
    if total > catalog_delta::DELTA_PAGE {
        let mut c = cache()
            .lock()
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "SYNC_BUSY"))?;
        c.values.retain(|_, v| v.created.elapsed() < TTL);
        if c.values.len() >= MAX_SNAPSHOTS
            || c.values
                .values()
                .map(|value| value.serialized_bytes)
                .sum::<usize>()
                .saturating_add(serialized_bytes)
                > catalog_delta::DELTA_MAX_BYTES
        {
            return Err(ApiError(StatusCode::TOO_MANY_REQUESTS, "SYNC_BUSY"));
        }
        c.values.insert(
            id.clone(),
            Baseline {
                server,
                profile: profile.clone(),
                revision: revision.clone(),
                created: Instant::now(),
                snapshot: stored,
                serialized_bytes,
            },
        );
    }
    // A rejected transfer must not manufacture a durable cursor.  Once an
    // active transfer has been admitted, persist its compact signature state.
    let persisted = state
        .database
        .call({
            let profile = profile.clone();
            let revision = revision.clone();
            let snapshot = snapshot.clone();
            move |store| store.save_catalog_delta_checkpoint(&profile, &revision, &snapshot)
        })
        .await;
    if let Err(error) = persisted {
        if let Ok(mut c) = cache().lock() {
            c.values.remove(&id);
        }
        return Err(error);
    }
    Ok(Json(
        serde_json::to_value(CatalogChanges {
            revision,
            unchanged: false,
            reset,
            snapshot_id: Some(id),
            total,
            items: first,
            deleted_ids,
            sagas,
            playlists,
            series_revisions,
            season_metadata,
            has_more: total > catalog_delta::DELTA_PAGE,
            next_offset: (total > catalog_delta::DELTA_PAGE).then_some(catalog_delta::DELTA_PAGE),
        })
        .unwrap(),
    ))
}
async fn page(
    State(state): State<AppState>,
    Path((profile, id)): Path<(String, String)>,
    Query(query): Query<Page>,
) -> Result<Json<Value>, ApiError> {
    let server = state
        .certificate
        .lock()
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "SYNC_BUSY"))?
        .server_id
        .clone();
    // Revalidate ownership/liveness on every page. A deleted profile must not
    // continue to page an immutable transfer that happened to be cached.
    state
        .database
        .call({
            let profile = profile.clone();
            move |store| store.sync_revision(&profile).map(|_| ())
        })
        .await?;
    let offset = query.offset.unwrap_or(0);
    let value = {
        let mut c = cache()
            .lock()
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "SYNC_BUSY"))?;
        c.values.retain(|_, v| v.created.elapsed() < TTL);
        c.values.get(&id).cloned()
    }
    .ok_or(ApiError(StatusCode::NOT_FOUND, "SYNC_SNAPSHOT_NOT_FOUND"))?;
    if value.server != server
        || value.profile != profile
        || offset == 0
        || offset >= value.snapshot.items.len()
        || offset % catalog_delta::DELTA_PAGE != 0
    {
        return Err(ApiError(StatusCode::NOT_FOUND, "SYNC_SNAPSHOT_NOT_FOUND"));
    }
    let end = (offset + catalog_delta::DELTA_PAGE).min(value.snapshot.items.len());
    let has_more = end < value.snapshot.items.len();
    // Completed transfers no longer occupy a concurrent-transfer slot. A lost
    // last-page response may be restarted with POST, retaining the old mirror.
    if !has_more && let Ok(mut c) = cache().lock() {
        c.values.remove(&id);
    }
    Ok(Json(
        serde_json::to_value(CatalogChanges {
            revision: value.revision,
            unchanged: false,
            reset: false,
            snapshot_id: Some(id),
            total: value.snapshot.items.len(),
            items: value.snapshot.items[offset..end].to_vec(),
            deleted_ids: Vec::new(),
            sagas: Vec::new(),
            playlists: Vec::new(),
            series_revisions: Default::default(),
            season_metadata: Default::default(),
            has_more,
            next_offset: has_more.then_some(end),
        })
        .unwrap(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn item(id: &str, title: &str, revision: u64) -> SyncMedia {
        SyncMedia {
            media: serde_json::from_value(json!({
                "id":id,"path":format!("/{id}"),"title":title,"category":"film",
                "year":null,"series":null,"season":null,"episode":null,"genres":[],"overview":"","poster":null,"saga":null,
                "watched":false,"rating":null,"available":true,"added_at":1,"tmdb_id":null
            }))
            .unwrap(),
            revision,
        }
    }
    fn snapshot(items: Vec<SyncMedia>) -> SyncSnapshot {
        SyncSnapshot {
            items,
            sagas: vec![],
            playlists: vec![],
            series_revisions: Default::default(),
            season_metadata: Default::default(),
            serialized_bytes: 0,
            captured_revision: (1, 1),
        }
    }
    #[test]
    fn delta_reports_one_add_update_and_delete() {
        let before = snapshot(vec![
            item("00000000-0000-0000-0000-000000000001", "old", 1),
            item("00000000-0000-0000-0000-000000000002", "gone", 1),
        ]);
        let after = snapshot(vec![
            item("00000000-0000-0000-0000-000000000001", "new", 2),
            item("00000000-0000-0000-0000-000000000003", "added", 1),
        ]);
        let (upserts, deleted) =
            changed(&catalog_delta::checkpoint(&before).unwrap(), &after).unwrap();
        assert_eq!(upserts.len(), 2);
        assert_eq!(deleted, vec!["00000000-0000-0000-0000-000000000002"]);
    }
    #[test]
    fn durable_revision_has_no_process_nonce() {
        assert_eq!(
            catalog_delta::revision("server", "profile", (9, 3)),
            catalog_delta::revision("server", "profile", (9, 3))
        );
    }
    #[test]
    fn generation_survives_restart_and_rejects_symlinks() {
        let temp = tempfile::tempdir().unwrap();
        let database = temp.path().join("catalog.sqlite");
        let first = database_generation(&database).unwrap();
        assert_eq!(first, database_generation(&database).unwrap());
        let other = temp.path().join("other.sqlite");
        std::os::unix::fs::symlink(
            database.with_extension("sync-generation"),
            other.with_extension("sync-generation"),
        )
        .unwrap();
        assert!(database_generation(&other).is_err());
    }
}
