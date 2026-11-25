use crate::{
    api::{ApiError, AppState},
    media_stream::{FileVersion, io_job, open_indexed},
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use chill_catalog::model::Media;
use chill_domain::segments::{Segment, SegmentEdit, SegmentScope, valid_segments};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{fs::File, path::PathBuf};

pub fn routes() -> Router<AppState> {
    Router::new().route(
        "/profiles/{profile}/media/{media}/segments",
        get(view).put(update),
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    scope: Option<SegmentScope>,
}

struct Snapshot {
    item: Media,
    path: PathBuf,
    library_revision: u64,
    file: File,
    version: FileVersion,
}

async fn snapshot(state: &AppState, profile: &str, media: &str) -> Result<Snapshot, ApiError> {
    let p = profile.to_owned();
    let m = media.to_owned();
    let (item, path) = state
        .database
        .call(move |store| Ok((store.media(&p, &m)?, store.media_path(&m)?)))
        .await?;
    let settings = state.library.lock().map_err(|_| unavailable())?.clone();
    let root = settings.root_for_path(&path).ok_or_else(unavailable)?;
    let target = path.clone();
    let (file, version) = io_job(&state.playback, move || open_indexed(&root, &target)).await?;
    Ok(Snapshot {
        item,
        path,
        library_revision: settings.revision,
        file,
        version,
    })
}

fn unavailable() -> ApiError {
    ApiError(
        StatusCode::SERVICE_UNAVAILABLE,
        "PLAYBACK_SEGMENTS_UNAVAILABLE",
    )
}

pub(crate) async fn probe(
    state: &AppState,
    file: File,
    version: FileVersion,
) -> Result<(Vec<Segment>, Option<u64>), ApiError> {
    let permit = state
        .playback
        .duration_io
        .clone()
        .try_acquire_owned()
        .map_err(|_| unavailable())?;
    io_job(&state.playback, move || {
        let _permit = permit;
        let duration = crate::media_duration::duration_ms(&file);
        let segments = crate::media_duration::playback_segments(&file, duration);
        let observed = FileVersion::from_metadata(&file.metadata()?)?;
        if observed != version {
            return Err(std::io::Error::other("changed source"));
        }
        Ok((segments, duration))
    })
    .await
}

pub(crate) async fn resolved(
    state: &AppState,
    profile: String,
    item: Media,
    version: String,
    chapters: Vec<Segment>,
    duration: Option<u64>,
) -> Result<Vec<Segment>, ApiError> {
    if item.category != "serie" {
        return Ok(chapters);
    }
    state
        .database
        .call(move |store| {
            let episode = store
                .manual_segments(&profile, &item.id, SegmentScope::Episode, &version)?
                .0;
            let season = if item.season.is_some() {
                store
                    .manual_segments(&profile, &item.id, SegmentScope::Season, &version)?
                    .0
            } else {
                Vec::new()
            };
            let mut segments = Vec::new();
            for item in episode.into_iter().chain(season).chain(chapters) {
                if duration.is_some_and(|length| item.end_ms > length) {
                    continue;
                }
                segments.push(item);
                if !valid_segments(&segments) {
                    segments.pop();
                }
            }
            Ok(segments)
        })
        .await
}

async fn view(
    State(state): State<AppState>,
    Path((profile, media)): Path<(String, String)>,
    Query(selected): Query<Selection>,
) -> Result<Json<Value>, ApiError> {
    let Snapshot {
        item,
        file,
        version,
        ..
    } = snapshot(&state, &profile, &media).await?;
    let token = version.etag(0);
    let (chapters, duration) = probe(&state, file, version).await?;
    let segments = resolved(
        &state,
        profile.clone(),
        item,
        token.clone(),
        chapters,
        duration,
    )
    .await?;
    let mut result = json!({"segments":segments,"duration_ms":duration,"version":token});
    if let Some(scope) = selected.scope {
        let (manual, revision) = state
            .database
            .call(move |store| store.manual_segments(&profile, &media, scope, &token))
            .await?;
        result["manual"] = json!(manual);
        result["revision"] = json!(revision);
    }
    Ok(Json(result))
}

async fn update(
    State(state): State<AppState>,
    Path((profile, media)): Path<(String, String)>,
    Query(selected): Query<Selection>,
    Json(edit): Json<SegmentEdit>,
) -> Result<Json<Value>, ApiError> {
    let invalid = || {
        ApiError(
            StatusCode::UNPROCESSABLE_ENTITY,
            "PLAYBACK_SEGMENTS_INVALID",
        )
    };
    let scope = selected.scope.ok_or_else(invalid)?;
    if edit.expected_revision >= i64::MAX as u64 || !valid_segments(&edit.segments) {
        return Err(invalid());
    }
    let Snapshot {
        file,
        version,
        path,
        library_revision,
        ..
    } = snapshot(&state, &profile, &media).await?;
    if edit.version != version.etag(0) {
        return Err(ApiError(StatusCode::CONFLICT, "PLAYBACK_SEGMENTS_CHANGED"));
    }
    if !edit.segments.is_empty() {
        let (_, duration) = probe(&state, file, version).await?;
        if duration.is_none_or(|length| edit.segments.iter().any(|item| item.end_ms > length)) {
            return Err(invalid());
        }
    }
    let library = state.library.clone();
    let saved = state
        .database
        .call(move |store| {
            let settings = library.lock().map_err(|_| "INVALID_OPERATION")?;
            if settings.revision != library_revision || store.media_path(&media)? != path {
                return Ok(false);
            }
            match store.save_manual_segments(&profile, &media, scope, edit) {
                Ok(()) => Ok(true),
                Err(reason) if reason == "PLAYBACK_SEGMENTS_CHANGED" => Ok(false),
                Err(reason) => Err(reason),
            }
        })
        .await?;
    if !saved {
        return Err(ApiError(StatusCode::CONFLICT, "PLAYBACK_SEGMENTS_CHANGED"));
    }
    Ok(Json(json!({"saved":true})))
}
