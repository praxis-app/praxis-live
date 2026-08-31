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
// Rejection limits: an upload past any of these is refused outright, because
// it is too large to decode safely. They also bound the memory a single
// decode can claim, so keep the allocation in step with the pixel count.
//
// Compression limits: an accepted upload larger than the threshold is
// downscaled to the target dimension and re-encoded before it is stored, so
// the rejection limits are a ceiling on what we accept, not on what we keep.
//
// Note that `MAX_IMAGE_BYTES` also sizes the multipart body limits in
// `common::request`, and is mirrored by the client-side check in the view's
// `image.utilts.ts`. Raising it widens both.

/// Longest edge accepted, in pixels.
const MAX_IMAGE_DIMENSION: u32 = 20_000;
/// Total pixels accepted, which bounds the decoded size of skewed images.
const MAX_IMAGE_PIXELS: u64 = 50_000_000;
/// Decode memory ceiling: `MAX_IMAGE_PIXELS` at four bytes per pixel.
const MAX_DECODE_ALLOCATION: u64 = 200 * 1024 * 1024;
/// Largest upload accepted before compression, in bytes.
pub(super) const MAX_IMAGE_BYTES: usize = 20 * 1024 * 1024;

/// Longest edge kept in storage; anything larger is downscaled to fit.
const COMPRESSION_TARGET_DIMENSION: u32 = 2_560;
/// Uploads at or below this size are stored exactly as received.
const COMPRESSION_THRESHOLD_BYTES: usize = 1024 * 1024;
/// Quality used when re-encoding opaque images as JPEG.
const JPEG_QUALITY: u8 = 82;

/// Validates an upload and, when it is larger than we want to store,
/// downscales and re-encodes it. Returns the bytes to persist.
///
/// Opaque images are re-encoded as JPEG, images with an alpha channel stay
/// PNG, and GIF and WebP are stored untouched. See [`is_compressible`].
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

    // Re-encoding can inflate already well-compressed uploads.
    Ok(if compressed.len() < bytes.len() {
        compressed
    } else {
        bytes
    })
}

/// Off-runtime wrapper: decoding and resizing are CPU-bound.
pub(crate) async fn normalize_raster_async(
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

/// Applies the rejection limits and returns the decoded image alongside its
/// format, so callers that go on to resize do not decode a second time.
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

/// GIFs may be animated and WebP has no encoder in `image`, so both are
/// stored as uploaded.
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

    /// Detailed enough that the PNG does not compress down to nothing;
    /// a smaller `detail` produces a larger file.
    fn encode_photo_png(width: u32, height: u32, detail: u32) -> Vec<u8> {
        let mut pixels = vec![0u8; (width as usize) * (height as usize) * 3];
        for y in 0..height {
            for x in 0..width {
                let value = (((x / detail) ^ (y / detail)) % 251) as u8;
                let index = ((y * width + x) * 3) as usize;
                pixels[index] = value;
                pixels[index + 1] = value;
                pixels[index + 2] = value;
            }
        }
        let mut bytes = Vec::new();
        PngEncoder::new(&mut bytes)
            .write_image(&pixels, width, height, ExtendedColorType::Rgb8)
            .unwrap();
        bytes
    }

    #[test]
    fn accepts_high_resolution_image_under_size_limit() {
        let bytes = encode_png(9_000, 5_000);
        assert!(bytes.len() < MAX_IMAGE_BYTES);

        let result = inspect(&bytes, "Message image");
        assert!(result.is_ok(), "{:?}", result.err().map(|e| e.to_string()));
    }

    #[test]
    fn oversized_image_error_avoids_pixel_jargon() {
        let bytes = encode_png(MAX_IMAGE_DIMENSION + 1, 200);
        let error = inspect(&bytes, "Message image")
            .expect_err("expected oversized image to be rejected");
        let message = error.to_string();
        assert!(!message.contains("pixel"), "{message}");
    }

    #[test]
    fn compresses_image_that_exceeds_storage_target() {
        let bytes = encode_photo_png(4_000, 3_000, 4);
        let original_len = bytes.len();

        let normalized = normalize_raster(bytes, "Message image").unwrap();

        assert!(
            normalized.len() < original_len,
            "expected compression, got {} from {original_len}",
            normalized.len()
        );
        let (width, height) =
            dimensions(&normalized, ImageFormat::Jpeg, "Message image")
                .unwrap();
        assert_eq!(width, COMPRESSION_TARGET_DIMENSION);
        assert_eq!(height, 1_920);
    }

    #[test]
    fn accepts_image_between_old_and_new_size_limits() {
        // Previously rejected outright at 8 MB; now compressed instead.
        let bytes = encode_photo_png(6_000, 4_500, 2);
        assert!(bytes.len() > 8 * 1024 * 1024);
        assert!(bytes.len() < MAX_IMAGE_BYTES);

        let normalized = normalize_raster(bytes, "Message image").unwrap();
        assert!(normalized.len() < 8 * 1024 * 1024);
    }

    #[test]
    fn rejects_image_beyond_compression_ceiling() {
        let bytes = vec![0u8; MAX_IMAGE_BYTES + 1];
        let error = normalize_raster(bytes, "Message image")
            .expect_err("expected rejection past the compression ceiling");
        assert!(error.to_string().contains("20 MB"), "{error}");
    }

    #[test]
    fn preserves_transparency_when_compressing() {
        let width = 3_000;
        let height = 3_000;
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        for y in 0..height {
            for x in 0..width {
                let value = (((x / 4) ^ (y / 4)) % 251) as u8;
                let index = ((y * width + x) * 4) as usize;
                pixels[index] = value;
                pixels[index + 1] = value;
                pixels[index + 2] = value;
                pixels[index + 3] = 128;
            }
        }
        let mut bytes = Vec::new();
        PngEncoder::new(&mut bytes)
            .write_image(&pixels, width, height, ExtendedColorType::Rgba8)
            .unwrap();

        let normalized = normalize_raster(bytes, "User image").unwrap();

        assert_eq!(supported_format(&normalized), Some(ImageFormat::Png));
        assert!(decode(&normalized, ImageFormat::Png, "User image")
            .unwrap()
            .color()
            .has_alpha());
    }

    #[test]
    fn leaves_small_images_untouched() {
        let bytes = encode_photo_png(400, 300, 4);
        let normalized =
            normalize_raster(bytes.clone(), "Message image").unwrap();
        assert_eq!(normalized, bytes);
    }

    #[test]
    fn stores_gifs_as_uploaded() {
        let mut bytes = Vec::new();
        image::codecs::gif::GifEncoder::new(&mut bytes)
            .encode(
                &vec![0u8; (3_000 * 3_000 * 4) as usize],
                3_000,
                3_000,
                image::ExtendedColorType::Rgba8,
            )
            .unwrap();

        let normalized =
            normalize_raster(bytes.clone(), "Message image").unwrap();
        assert_eq!(normalized, bytes);
    }
}
