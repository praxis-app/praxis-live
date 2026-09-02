use axum::{
    body::Body,
    http::{header, Response, StatusCode},
};
use image::{
    codecs::jpeg::JpegEncoder, imageops::FilterType, DynamicImage, ImageFormat,
    ImageReader, Limits,
};
use std::io::Cursor;

use super::{ApiError, AppResult};

// Uploads pass through two tiers of limits.
//
// Rejection limits refuse an upload outright as too large to decode safely,
// and bound the memory one decode can claim: `MAX_DECODE_ALLOCATION` is
// `MAX_IMAGE_PIXELS` at four bytes per pixel, so the two move together.
// `MAX_IMAGE_BYTES` also sizes the multipart body limits in `common::request`
// and is mirrored by the client check in the view's `image.utilts.ts`.
//
// Compression limits apply to uploads we accept: anything past the threshold
// is downscaled to the target dimension and re-encoded before storage, so the
// rejection limits cap what we accept, not what we keep. Opaque images become
// JPEG and images with alpha stay PNG; GIF may be animated and WebP has no
// encoder in `image`, so both are stored as uploaded. A re-encode that comes
// out larger than the original is discarded.
//
// `inspect` returns the image it decoded, so normalization decodes only once.

const MAX_IMAGE_DIMENSION: u32 = 20_000;
const MAX_IMAGE_PIXELS: u64 = 50_000_000;
const MAX_DECODE_ALLOCATION: u64 = 200 * 1024 * 1024;
pub(super) const MAX_IMAGE_BYTES: usize = 20 * 1024 * 1024;

const COMPRESSION_TARGET_DIMENSION: u32 = 2_560;
const COMPRESSION_THRESHOLD_BYTES: usize = 1024 * 1024;
const JPEG_QUALITY: u8 = 82;

pub(crate) fn normalize_raster(
    bytes: Vec<u8>,
    label: &str,
) -> AppResult<Vec<u8>> {
    let (format, image) = inspect(&bytes, label)?;
    let (width, height) = (image.width(), image.height());

    let oversized = bytes.len() > COMPRESSION_THRESHOLD_BYTES
        || width > COMPRESSION_TARGET_DIMENSION
        || height > COMPRESSION_TARGET_DIMENSION;
    if !oversized || !is_compressible(format) {
        return Ok(bytes);
    }

    let image = if width > COMPRESSION_TARGET_DIMENSION
        || height > COMPRESSION_TARGET_DIMENSION
    {
        image.resize(
            COMPRESSION_TARGET_DIMENSION,
            COMPRESSION_TARGET_DIMENSION,
            FilterType::Lanczos3,
        )
    } else {
        image
    };

    let compressed = encode(&image).map_err(|error| {
        tracing::error!("failed to compress {label}: {error}");
        invalid_image(label, "could not be processed")
    })?;

    Ok(if compressed.len() < bytes.len() {
        compressed
    } else {
        bytes
    })
}

/// Validates an upload and compresses it for storage, decoding on a blocking
/// thread so the async runtime is not stalled.
pub(crate) async fn normalize_upload(
    bytes: Vec<u8>,
    label: &'static str,
) -> AppResult<Vec<u8>> {
    tokio::task::spawn_blocking(move || normalize_raster(bytes, label))
        .await
        .map_err(|error| {
            tracing::error!("image normalization task failed: {error}");
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error.",
            )
        })?
}

fn inspect(
    bytes: &[u8],
    label: &str,
) -> AppResult<(ImageFormat, DynamicImage)> {
    if bytes.is_empty() {
        return Err(invalid_image(label, "is required"));
    }
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(invalid_image(
            label,
            &format!(
                "must be no larger than {} MB",
                MAX_IMAGE_BYTES / (1024 * 1024)
            ),
        ));
    }

    let format = supported_format(bytes).ok_or_else(|| {
        invalid_image(label, "must be a PNG, JPEG, GIF, or WebP image")
    })?;
    let (width, height) = dimensions(bytes, format, label)?;
    let too_large = width > MAX_IMAGE_DIMENSION
        || height > MAX_IMAGE_DIMENSION
        || u64::from(width) * u64::from(height) > MAX_IMAGE_PIXELS;
    if too_large {
        return Err(invalid_image(
            label,
            "is too large to process. Try uploading a resized version",
        ));
    }

    let image = decode(bytes, format, label)?;

    Ok((format, image))
}

fn dimensions(
    bytes: &[u8],
    format: ImageFormat,
    label: &str,
) -> AppResult<(u32, u32)> {
    ImageReader::with_format(Cursor::new(bytes), format)
        .into_dimensions()
        .map_err(|_| invalid_image(label, "is not a valid image"))
}

fn decode(
    bytes: &[u8],
    format: ImageFormat,
    label: &str,
) -> AppResult<DynamicImage> {
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
    limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_ALLOCATION);
    reader.limits(limits);
    reader
        .decode()
        .map_err(|_| invalid_image(label, "is not a valid image"))
}

fn is_compressible(format: ImageFormat) -> bool {
    matches!(format, ImageFormat::Png | ImageFormat::Jpeg)
}

fn encode(image: &DynamicImage) -> Result<Vec<u8>, image::ImageError> {
    let mut bytes = Vec::new();
    if image.color().has_alpha() {
        image.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)?;
    } else {
        let rgb = image.to_rgb8();
        JpegEncoder::new_with_quality(&mut bytes, JPEG_QUALITY)
            .encode_image(&rgb)?;
    }
    Ok(bytes)
}

pub(crate) fn safe_image_response(bytes: Vec<u8>) -> AppResult<Response<Body>> {
    let detected_content_type =
        supported_format(&bytes).and_then(supported_content_type);
    let content_type =
        detected_content_type.unwrap_or("application/octet-stream");
    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff");
    if detected_content_type.is_none() {
        builder = builder.header(header::CONTENT_DISPOSITION, "attachment");
    }
    builder.body(Body::from(bytes)).map_err(|error| {
        tracing::error!("failed to build image response: {error}");
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error.",
        )
    })
}

fn supported_format(bytes: &[u8]) -> Option<ImageFormat> {
    image::guess_format(bytes)
        .ok()
        .filter(|format| supported_content_type(*format).is_some())
}

fn supported_content_type(format: ImageFormat) -> Option<&'static str> {
    match format {
        ImageFormat::Png => Some("image/png"),
        ImageFormat::Jpeg => Some("image/jpeg"),
        ImageFormat::Gif => Some("image/gif"),
        ImageFormat::WebP => Some("image/webp"),
        _ => None,
    }
}

fn invalid_image(label: &str, requirement: &str) -> ApiError {
    ApiError::new(
        StatusCode::UNPROCESSABLE_ENTITY,
        format!("{label} {requirement}."),
    )
}
