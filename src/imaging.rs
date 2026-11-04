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
    if bytes.len() >= 6 && &bytes[0..6] == b"GIF87a"
        || bytes.len() >= 6 && &bytes[0..6] == b"GIF89a"
    {
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

fn decode(bytes: &[u8]) -> AppResult<DynamicImage> {
    image::load_from_memory(bytes).map_err(|e| {
        tracing::warn!(error = %e, "image decode failed");
        AppError::UnsupportedFormat
    })
}

/// Width, height, and a WebP thumbnail from a single decode.
/// The thumbnail fits inside `max_edge` on both sides. Animated GIFs collapse
/// to their first frame here; the caller stores the original bytes unchanged.
pub struct Preview {
    pub width: u32,
    pub height: u32,
    pub thumbnail: Vec<u8>,
}

pub fn preview(bytes: &[u8], max_edge: u32) -> AppResult<Preview> {
    let img = decode(bytes)?;
    let (width, height) = img.dimensions();
    let thumb = if width > max_edge || height > max_edge {
        img.resize(max_edge, max_edge, image::imageops::FilterType::Triangle)
    } else {
        img
    };
    let mut thumbnail = Vec::with_capacity(8 * 1024);
    thumb
        .write_to(&mut Cursor::new(&mut thumbnail), ImageFormat::WebP)
        .map_err(|e| {
            tracing::warn!(error = %e, "thumbnail encode failed");
            AppError::UnsupportedFormat
        })?;
    Ok(Preview {
        width,
        height,
        thumbnail,
    })
}

pub const THUMBNAIL_CONTENT_TYPE: &str = "image/webp";

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let img = image::RgbImage::from_pixel(width, height, image::Rgb([10, 20, 30]));
        let mut bytes = Vec::new();
        DynamicImage::ImageRgb8(img)
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .unwrap();
        bytes
    }

    #[test]
    fn preview_shrinks_the_long_edge() {
        let preview = preview(&png(8, 4), 2).unwrap();
        assert_eq!((preview.width, preview.height), (8, 4));
        let thumb = image::load_from_memory(&preview.thumbnail).unwrap();
        assert!(thumb.width() <= 2 && thumb.height() <= 2);
        assert!(thumb.width() == 2 || thumb.height() == 2);
    }

    #[test]
    fn preview_keeps_small_images() {
        let preview = preview(&png(2, 2), 480).unwrap();
        let thumb = image::load_from_memory(&preview.thumbnail).unwrap();
        assert_eq!((thumb.width(), thumb.height()), (2, 2));
    }
}
