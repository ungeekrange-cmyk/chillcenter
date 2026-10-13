//! Staged, immutable administration exports.  Import is deliberately separate.
use crate::api::{ApiError, AppState};
use axum::{
    Json, Router,
    body::Body,
    extract::{Path, State},
    http::{StatusCode, header},
    response::Response,
    routing::{get, post},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::PathBuf};
use tokio_util::io::ReaderStream;
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

const MAX_COMPRESSED: u64 = 1024 * 1024 * 1024;
const MAX_DECOMPRESSED: u64 = 2 * 1024 * 1024 * 1024;
const MAX_ENTRIES: u64 = 20_000;
const MAX_MANIFEST: usize = 64 * 1024 * 1024;

#[derive(Serialize)]
struct ExportReady {
    export_id: String,
    bytes: u64,
    entries: u64,
    state: &'static str,
}

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/admin/exports", post(create))
        .route("/api/v1/admin/exports/{id}", get(download))
}

fn export_dir(state: &AppState) -> Result<PathBuf, ApiError> {
    let path = state.config.data_dir.join("exports");
    fs::create_dir_all(&path)
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
    Ok(path)
}

async fn create(State(state): State<AppState>) -> Result<Json<ExportReady>, ApiError> {
    let id = uuid::Uuid::new_v4().to_string();
    let dir = export_dir(&state)?;
    let partial = dir.join(format!("{id}.partial"));
    let ready = dir.join(format!("{id}.zip"));
    let data_stage = dir.join(format!("{id}.data.partial"));
    let poster_root = state.config.data_dir.join("posters");
    let writer_path = data_stage.clone();
    let export_id = id.clone();
    let writer_poster_root = poster_root.clone();
    let posters = state
        .database
        .call(move |store| {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&writer_path)
                .map_err(|_| "Export de staging inaccessible.".to_string())?;
            let posters = store.write_archive_json(&mut file, &export_id, &writer_poster_root)?;
            file.flush()
                .map_err(|_| "Export de staging inaccessible.")?;
            file.sync_all()
                .map_err(|_| "Export de staging inaccessible.")?;
            Ok(posters)
        })
        .await?;
    let data_size = fs::metadata(&data_stage)
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?
        .len();
    if data_size > MAX_DECOMPRESSED {
        let _ = fs::remove_file(&data_stage);
        return Err(ApiError(StatusCode::PAYLOAD_TOO_LARGE, "EXPORT_TOO_LARGE"));
    }
    let result = (|| -> Result<(u64, u64), ApiError> {
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&partial)
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        let mut checksums =
            vec![serde_json::json!({"reference":"data.json","sha256":hash_file(&data_stage)?})];
        let mut poster_bytes = Vec::new();
        for poster in &posters {
            let source = poster_root.join(&poster.relative);
            let bytes = fs::read(&source)
                .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_POSTER"))?;
            if bytes.len() as u64 > chill_domain::store::MAX_EXPORT_POSTER_BYTES {
                return Err(ApiError(StatusCode::PAYLOAD_TOO_LARGE, "EXPORT_TOO_LARGE"));
            }
            checksums.push(serde_json::json!({"reference":poster.reference,"sha256":format!("{:x}",Sha256::digest(&bytes))}));
            poster_bytes.push((poster.reference.clone(), bytes));
        }
        let manifest = serde_json::to_vec(&serde_json::json!({"format_version":1,"export_id":id,"entries":checksums,"videos":false,"leases":false,"secrets":false})).map_err(|_| ApiError(StatusCode::UNPROCESSABLE_ENTITY, "EXPORT_INVALID"))?;
        if manifest.len() > MAX_MANIFEST {
            return Err(ApiError(StatusCode::PAYLOAD_TOO_LARGE, "EXPORT_TOO_LARGE"));
        }
        zip.start_file("manifest.json", options)
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
        zip.write_all(&manifest)
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
        zip.start_file("data.json", options)
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
        let mut data_file = fs::File::open(&data_stage)
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
        std::io::copy(&mut data_file, &mut zip)
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
        for (reference, bytes) in poster_bytes {
            zip.start_file(reference, options)
                .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
            zip.write_all(&bytes)
                .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
        }
        let mut file = zip
            .finish()
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
        file.flush()
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
        file.sync_all()
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
        let size = file
            .metadata()
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?
            .len();
        if size > MAX_COMPRESSED || (2 + posters.len() as u64) > MAX_ENTRIES {
            return Err(ApiError(StatusCode::PAYLOAD_TOO_LARGE, "EXPORT_TOO_LARGE"));
        }
        Ok((size, 2 + posters.len() as u64))
    })();
    match result {
        Ok((bytes, entries)) => {
            fs::rename(&partial, &ready)
                .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
            fs::File::open(&dir)
                .and_then(|directory| directory.sync_all())
                .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
            let _ = fs::remove_file(&data_stage);
            Ok(Json(ExportReady {
                export_id: id,
                bytes,
                entries,
                state: "ready",
            }))
        }
        Err(error) => {
            let _ = fs::remove_file(&partial);
            let _ = fs::remove_file(&data_stage);
            Err(error)
        }
    }
}

fn hash_file(path: &std::path::Path) -> Result<String, ApiError> {
    let mut file = fs::File::open(path)
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = std::io::Read::read(&mut file, &mut buffer)
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "EXPORT_STORAGE"))?;
        if n == 0 {
            break;
        };
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

async fn download(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    if uuid::Uuid::parse_str(&id).is_err() {
        return Err(ApiError(StatusCode::NOT_FOUND, "EXPORT_NOT_FOUND"));
    }
    let path = export_dir(&state)?.join(format!("{id}.zip"));
    let file = tokio::fs::File::open(path)
        .await
        .map_err(|_| ApiError(StatusCode::NOT_FOUND, "EXPORT_NOT_FOUND"))?;
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/zip")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=chillcenter-{id}.zip"),
        )
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from_stream(ReaderStream::new(file)))
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "EXPORT_STORAGE"))
}
