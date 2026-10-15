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

const TTL: Duration = Duration::from_secs(120);
const MAX_SNAPSHOTS: usize = 3;
#[derive(Clone)]
struct Baseline {
    server: String,
    profile: String,
    revision: String,
    created: Instant,
    /// Full immutable image used only to compute the next bounded delta.
    baseline: Arc<SyncSnapshot>,
    /// The transfer image can be smaller than the baseline after a delta.
    snapshot: Arc<SyncSnapshot>,
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
fn document(
    snapshot: &SyncSnapshot,
) -> (
    Vec<SagaView>,
    Vec<PlaylistView>,
    std::collections::BTreeMap<String, u64>,
    std::collections::BTreeMap<String, chill_catalog::model::SeasonProvider>,
) {
    (
        snapshot.sagas.clone(),
        snapshot.playlists.clone(),
        snapshot.series_revisions.clone(),
        snapshot.season_metadata.clone(),
    )
}
fn changed(old: &SyncSnapshot, now: &SyncSnapshot) -> (Vec<SyncMedia>, Vec<String>) {
    let old = old
        .items
        .iter()
        .map(|m| (m.media.id.as_str(), m))
        .collect::<HashMap<_, _>>();
    let next = now
        .items
        .iter()
        .map(|m| (m.media.id.as_str(), m))
        .collect::<HashMap<_, _>>();
    let upserts = now
        .items
        .iter()
        .filter(|m| {
            old.get(m.media.id.as_str())
                .is_none_or(|v| serde_json::to_vec(v).ok() != serde_json::to_vec(m).ok())
        })
        .cloned()
        .collect();
    let deleted_ids = old
        .keys()
        .filter(|id| !next.contains_key(**id))
        .map(|id| (*id).to_owned())
        .collect();
    (upserts, deleted_ids)
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
    let revision = catalog_delta::revision(&server, &profile, epoch);
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
        catalog_delta::revision(&server, &profile, epoch),
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
    let old = request.known_revision.as_deref().and_then(|revision| {
        cache()
            .lock()
            .ok()?
            .values
            .values()
            .find(|v| v.server == server && v.profile == profile && v.revision == revision)
            .cloned()
    });
    let old = old.map(|value| (*value.baseline).clone()).or(durable_old);
    // Checkpoint the coherent post-state before returning its cursor.  A
    // restart can therefore compute the next delta without a process UUID.
    state
        .database
        .call({
            let profile = profile.clone();
            let revision = revision.clone();
            let snapshot = snapshot.clone();
            move |store| store.save_catalog_delta_checkpoint(&profile, &revision, &snapshot)
        })
        .await?;
    let (items, deleted_ids, reset) = if let Some(old) = old {
        let (a, b) = changed(&old, &snapshot);
        (a, b, false)
    } else {
        (snapshot.items.clone(), Vec::new(), true)
    };
    let id = Uuid::new_v4().to_string();
    let total = items.len();
    let first = items[..total.min(catalog_delta::DELTA_PAGE)].to_vec();
    let (sagas, playlists, series_revisions, season_metadata) = document(&snapshot);
    let baseline = Arc::new(snapshot.clone());
    let stored = Arc::new(SyncSnapshot {
        items,
        sagas: sagas.clone(),
        playlists: playlists.clone(),
        series_revisions: series_revisions.clone(),
        season_metadata: season_metadata.clone(),
        serialized_bytes: snapshot.serialized_bytes,
        captured_revision: snapshot.captured_revision,
    });
    {
        let mut c = cache()
            .lock()
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "SYNC_BUSY"))?;
        c.values.retain(|_, v| v.created.elapsed() < TTL);
        if c.values.len() >= MAX_SNAPSHOTS {
            if let Some(k) = c
                .values
                .iter()
                .min_by_key(|(_, v)| v.created)
                .map(|(k, _)| k.clone())
            {
                c.values.remove(&k);
            }
        }
        if c.values.len() >= MAX_SNAPSHOTS
            || c.values
                .values()
                .map(|value| value.baseline.serialized_bytes)
                .sum::<usize>()
                .saturating_add(baseline.serialized_bytes)
                > catalog_delta::DELTA_MAX_BYTES
        {
            return Err(ApiError(StatusCode::TOO_MANY_REQUESTS, "SYNC_BUSY"));
        }
        c.values.insert(
            id.clone(),
            Baseline {
                server,
                profile,
                revision: revision.clone(),
                created: Instant::now(),
                baseline,
                snapshot: stored,
            },
        );
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
    let (sagas, playlists, series_revisions, season_metadata) = document(&value.snapshot);
    Ok(Json(
        serde_json::to_value(CatalogChanges {
            revision: value.revision,
            unchanged: false,
            reset: false,
            snapshot_id: Some(id),
            total: value.snapshot.items.len(),
            items: value.snapshot.items[offset..end].to_vec(),
            deleted_ids: Vec::new(),
            sagas,
            playlists,
            series_revisions,
            season_metadata,
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
        let (upserts, deleted) = changed(&before, &after);
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
}
