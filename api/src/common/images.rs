use axum::{
    body::Body,
    http::{header, Response, StatusCode},
};
use image::{ImageFormat, ImageReader, Limits};
use std::io::Cursor;

use super::{ApiError, AppResult};

const MAX_IMAGE_DIMENSION: u32 = 20_000;
const MAX_IMAGE_PIXELS: u64 = 50_000_000;
const MAX_DECODE_ALLOCATION: u64 = 200 * 1024 * 1024;
pub(super) const MAX_IMAGE_BYTES: usize = 8 * 1024 * 1024;

pub(crate) fn validate_raster(bytes: &[u8], label: &str) -> AppResult<()> {
    if bytes.is_empty() {
        return Err(invalid_image(label, "is required"));
    }
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(invalid_image(label, "must be no larger than 8 MB"));
    }

    let format = supported_format(bytes).ok_or_else(|| {
        invalid_image(label, "must be a PNG, JPEG, GIF, or WebP image")
    })?;
    let (width, height) = ImageReader::with_format(Cursor::new(bytes), format)
        .into_dimensions()
        .map_err(|_| invalid_image(label, "is not a valid image"))?;
    let too_large = width > MAX_IMAGE_DIMENSION
        || height > MAX_IMAGE_DIMENSION
        || u64::from(width) * u64::from(height) > MAX_IMAGE_PIXELS;
    if too_large {
        return Err(invalid_image(
            label,
            "is too large to process. Try uploading a resized version",
        ));
    }

    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
    limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_ALLOCATION);
    reader.limits(limits);
    reader
        .decode()
        .map_err(|_| invalid_image(label, "is not a valid image"))?;

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use image::{codecs::png::PngEncoder, ExtendedColorType, ImageEncoder};

    fn encode_png(width: u32, height: u32) -> Vec<u8> {
        let pixels = vec![0u8; (width as usize) * (height as usize)];
        let mut bytes = Vec::new();
        PngEncoder::new(&mut bytes)
            .write_image(&pixels, width, height, ExtendedColorType::L8)
            .unwrap();
        bytes
    }

    #[test]
    fn accepts_high_resolution_image_under_size_limit() {
        let bytes = encode_png(9_000, 5_000);
        assert!(bytes.len() < MAX_IMAGE_BYTES);

        let result = validate_raster(&bytes, "Message image");
        assert!(result.is_ok(), "{:?}", result.err().map(|e| e.to_string()));
    }

    #[test]
    fn oversized_image_error_avoids_pixel_jargon() {
        let bytes = encode_png(MAX_IMAGE_DIMENSION + 1, 200);
        let error = validate_raster(&bytes, "Message image")
            .expect_err("expected oversized image to be rejected");
        let message = error.to_string();
        assert!(!message.contains("pixel"), "{message}");
    }
}
