//! Uploaded photos are decoded with allocation limits, resized, and re-encoded without metadata.
use crate::{AppState, Error, Result, db, model::Recipe};
use axum::{
    Json,
    body::to_bytes,
    extract::{Path, Request, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use image::{ImageFormat, ImageReader, Limits};
use std::{
    io::Cursor,
    sync::{Arc, LazyLock},
};
static PHOTO_WORKER: LazyLock<Arc<tokio::sync::Semaphore>> =
    LazyLock::new(|| Arc::new(tokio::sync::Semaphore::new(1)));
pub fn prepare(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| Error::bad("Could not read photo"))?;
    if !matches!(
        reader.format(),
        Some(ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::WebP)
    ) {
        return Err(Error::bad("Choose a JPEG, PNG, or WebP photo"));
    }
    let mut limits = Limits::default();
    limits.max_alloc = Some(32 * 1024 * 1024);
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    reader.limits(limits);
    let decoded = reader.decode().map_err(|_| {
        Error::bad("Photo could not be decoded or is too large; resize it to 4096 pixels or less")
    })?;
    let rgb = decoded.thumbnail(1200, 1200).to_rgb8();
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 82)
        .encode_image(&rgb)
        .map_err(|_| Error::bad("Could not encode photo"))?;
    if jpeg.len() > 2 * 1024 * 1024 {
        return Err(Error::bad("Photo is too large after resizing"));
    }
    Ok(jpeg)
}
pub async fn upload(
    State(s): State<AppState>,
    Path(id): Path<i64>,
    request: Request,
) -> Result<Json<Recipe>> {
    // Reject excess uploads before collecting their bodies into memory.
    let permit = PHOTO_WORKER
        .clone()
        .try_acquire_owned()
        .map_err(|_| Error {
            status: StatusCode::TOO_MANY_REQUESTS,
            message: "Photo processing is busy; retry shortly".into(),
        })?;
    let bytes = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        to_bytes(request.into_body(), 5 * 1024 * 1024),
    )
    .await
    .map_err(|_| Error {
        status: StatusCode::REQUEST_TIMEOUT,
        message: "Photo upload timed out".into(),
    })?
    .map_err(|_| Error {
        status: StatusCode::PAYLOAD_TOO_LARGE,
        message: "Photo body could not be read within the 5 MiB limit".into(),
    })?;
    let jpeg = tokio::task::spawn_blocking(move || {
        // Blocking work continues after request cancellation; retain capacity until it ends.
        let _permit = permit;
        prepare(&bytes)
    })
    .await
    .map_err(|_| Error::internal())??;
    Ok(Json(
        s.store.run(move |db| db::save_photo(db, id, jpeg)).await?,
    ))
}
pub async fn get(State(s): State<AppState>, Path(id): Path<i64>) -> Result<Response> {
    let bytes = s.store.run(move |db| db::photo(db, id)).await?;
    Ok((
        [
            (header::CONTENT_TYPE, "image/jpeg"),
            (header::CACHE_CONTROL, "private, no-store"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        bytes,
    )
        .into_response())
}
pub async fn delete(
    State(s): State<AppState>,
    Path(id): Path<i64>,
) -> Result<axum::http::StatusCode> {
    s.store.run(move |db| db::delete_photo(db, id)).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};

    #[tokio::test]
    async fn busy_upload_is_rejected_before_body_collection_and_capacity_is_reusable() {
        let state = AppState::new(db::Store::open(":memory:").unwrap(), None);
        let permit = PHOTO_WORKER.clone().try_acquire_owned().unwrap();
        let oversized = Request::new(Body::from(vec![0; 5 * 1024 * 1024 + 1]));
        let error = upload(State(state.clone()), Path(1), oversized)
            .await
            .unwrap_err();
        assert_eq!(error.status, StatusCode::TOO_MANY_REQUESTS);
        drop(permit);
        let oversized = Request::new(Body::from(vec![0; 5 * 1024 * 1024 + 1]));
        let error = upload(State(state.clone()), Path(1), oversized)
            .await
            .unwrap_err();
        assert_eq!(error.status, StatusCode::PAYLOAD_TOO_LARGE);
        let error = upload(State(state), Path(1), Request::new(Body::empty()))
            .await
            .unwrap_err();
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert!(PHOTO_WORKER.clone().try_acquire_owned().is_ok());
    }
}
