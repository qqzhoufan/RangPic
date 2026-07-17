use std::io::Cursor;

use image::{DynamicImage, GenericImageView, ImageFormat};
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};

/// Image kinds we accept. Stored as a DB enum string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Jpeg,
    Png,
    Gif,
    Webp,
    Bmp,
    Ico,
}

impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Format::Jpeg => "jpg",
            Format::Png => "png",
            Format::Gif => "gif",
            Format::Webp => "webp",
            Format::Bmp => "bmp",
            Format::Ico => "ico",
        }
    }
    pub fn content_type(self) -> &'static str {
        match self {
            Format::Jpeg => "image/jpeg",
            Format::Png => "image/png",
            Format::Gif => "image/gif",
            Format::Webp => "image/webp",
            Format::Bmp => "image/bmp",
            Format::Ico => "image/x-icon",
        }
    }
    pub fn as_db(self) -> &'static str {
        match self {
            Format::Jpeg => "jpeg",
            Format::Png => "png",
            Format::Gif => "gif",
            Format::Webp => "webp",
            Format::Bmp => "bmp",
            Format::Ico => "ico",
        }
    }
    pub fn parse_db(s: &str) -> Option<Self> {
        Some(match s {
            "jpeg" => Format::Jpeg,
            "png" => Format::Png,
            "gif" => Format::Gif,
            "webp" => Format::Webp,
            "bmp" => Format::Bmp,
            "ico" => Format::Ico,
            _ => return None,
        })
    }
}

/// Detect a supported image format from magic bytes. Falls back to extension sniffing.
pub fn detect(bytes: &[u8]) -> Option<Format> {
    if bytes.len() >= 3 && bytes[0..3] == [0xFF, 0xD8, 0xFF] {
        return Some(Format::Jpeg);
    }
    if bytes.len() >= 8 && bytes[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
        return Some(Format::Png);
    }
    if bytes.len() >= 6 && &bytes[0..6] == b"GIF87a" || bytes.len() >= 6 && &bytes[0..6] == b"GIF89a" {
        return Some(Format::Gif);
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some(Format::Webp);
    }
    if bytes.len() >= 2 && &bytes[0..2] == b"BM" {
        return Some(Format::Bmp);
    }
    if bytes.len() >= 4 && bytes[0..4] == [0x00, 0x00, 0x01, 0x00] {
        return Some(Format::Ico);
    }
    None
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

/// (width, height) of an image, if it can be decoded.
pub fn dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    let img = decode(bytes).ok()?;
    Some(img.dimensions())
}

fn decode(bytes: &[u8]) -> AppResult<DynamicImage> {
    image::load_from_memory(bytes).map_err(|e| {
        tracing::warn!(error = %e, "image decode failed");
        AppError::UnsupportedFormat
    })
}

/// Render a WebP thumbnail fit within `max` px on the long edge. Animated GIFs
/// collapse to their first frame in the thumbnail (the original is preserved).
pub fn make_thumbnail(bytes: &[u8], max: u32) -> AppResult<Vec<u8>> {
    let img = decode(bytes)?;
    let thumb = if img.width() > max || img.height() > max {
        img.resize(max, max, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };
    let mut out = Vec::with_capacity(8 * 1024);
    let mut cur = Cursor::new(&mut out);
    thumb
        .write_to(&mut cur, ImageFormat::WebP)
        .map_err(|e| {
            tracing::warn!(error = %e, "thumbnail encode failed");
            AppError::UnsupportedFormat
        })?;
    Ok(out)
}

pub const THUMBNAIL_CONTENT_TYPE: &str = "image/webp";
