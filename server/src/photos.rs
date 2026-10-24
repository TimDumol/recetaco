//! Uploaded photos are decoded with allocation limits, resized, and re-encoded without metadata.
use crate::{AppState, Error, Result, db, model::Recipe};
use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::header,
    response::{IntoResponse, Response},
};
use image::{ImageFormat, ImageReader, Limits};
use std::io::Cursor;
static PHOTO_WORKER: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);
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
    bytes: Bytes,
) -> Result<Json<Recipe>> {
    let _permit = PHOTO_WORKER
        .acquire()
        .await
        .map_err(|_| Error::internal())?;
    let jpeg = tokio::task::spawn_blocking(move || prepare(&bytes))
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
