// SPDX-License-Identifier: MPL-2.0

use crate::schema::MimeType;
use std::io::Read;
use thiserror::Error;
use tracing::debug;

const MAGIC_BUFFER_SIZE: usize = 64;

#[derive(Debug, Error)]
pub enum DetectionError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn detect_mime(data: &[u8]) -> MimeType {
    if data.len() >= 4 {
        if data[0..4] == [0x89, 0x50, 0x4E, 0x47] {
            debug!("Detected PNG by magic bytes");
            return MimeType::ImagePng;
        }

        if &data[0..3] == &[0xFF, 0xD8, 0xFF] {
            debug!("Detected JPEG by magic bytes");
            return MimeType::ImageJpeg;
        }

        if &data[0..2] == b"BM" {
            debug!("Detected BMP by magic bytes");
            return MimeType::ImageBmp;
        }

        if &data[0..4] == b"%PDF" {
            debug!("Detected PDF by magic bytes");
            return MimeType::Other("application/pdf".into());
        }

        if &data[0..4] == b"RIFF" && data.len() >= 8 {
            if &data[8..12] == b"WEBP" {
                debug!("Detected WebP by magic bytes");
                return MimeType::Other("image/webp".into());
            }
        }
    }

    if data.len() > 5 {
        let start = data
            .iter()
            .take_while(|&&b| b.is_ascii_whitespace())
            .count();
        if data.len() > start + 4 {
            let snippet = &data[start..];
            if snippet.starts_with(b"<svg") || snippet.starts_with(b"<?xml") {
                let s = std::str::from_utf8(&data[start..data.len().min(start + 100)]);
                if let Ok(s) = s {
                    if s.contains("<svg") {
                        debug!("Detected SVG by content");
                        return MimeType::ImageSvg;
                    }
                }
            }
        }
    }

    if let Ok(s) = std::str::from_utf8(data) {
        let lower = s.to_lowercase();
        if lower.starts_with("<!doctype html")
            || lower.starts_with("<html")
            || lower.contains("<head>")
            || lower.contains("<body>")
        {
            debug!("Detected HTML by content");
            return MimeType::TextHtml;
        }

        if lower.contains("file://") || lower.contains("http://") || lower.contains("https://") {
            if lower.lines().all(|line| {
                line.is_empty()
                    || line.starts_with('#')
                    || line.starts_with("file://")
                    || line.starts_with("http://")
                    || line.starts_with("https://")
            }) {
                debug!("Detected URI list by content");
                return MimeType::TextUri;
            }
        }

        debug!("Detected plain text (valid UTF-8)");
        return MimeType::TextPlain;
    }

    debug!("Unknown binary content, defaulting to octet-stream");
    MimeType::Other("application/octet-stream".into())
}

pub fn detect_mime_from_reader<R: Read>(mut reader: R) -> Result<MimeType, DetectionError> {
    let mut buffer = [0u8; MAGIC_BUFFER_SIZE];
    let bytes_read = reader.read(&mut buffer)?;
    Ok(detect_mime(&buffer[..bytes_read]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_png() {
        let png = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(detect_mime(&png), MimeType::ImagePng);
    }

    #[test]
    fn detect_jpeg() {
        let jpeg = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        assert_eq!(detect_mime(&jpeg), MimeType::ImageJpeg);
    }

    #[test]
    fn detect_bmp() {
        let bmp = b"BM\x00\x00\x00\x00";
        assert_eq!(detect_mime(bmp), MimeType::ImageBmp);
    }

    #[test]
    fn detect_svg() {
        let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><circle/></svg>";
        assert_eq!(detect_mime(svg), MimeType::ImageSvg);
    }

    #[test]
    fn detect_svg_with_xml_decl() {
        let svg = b"<?xml version=\"1.0\"?>\n<svg><rect/></svg>";
        assert_eq!(detect_mime(svg), MimeType::ImageSvg);
    }

    #[test]
    fn detect_html() {
        let html = b"<!DOCTYPE html><html><body>Hello</body></html>";
        assert_eq!(detect_mime(html), MimeType::TextHtml);
    }

    #[test]
    fn detect_text() {
        let text = b"Hello, world!";
        assert_eq!(detect_mime(text), MimeType::TextPlain);
    }

    #[test]
    fn detect_uri_list() {
        let uris = b"file:///home/user/doc.txt\nhttps://example.com";
        assert_eq!(detect_mime(uris), MimeType::TextUri);
    }

    #[test]
    fn detect_unknown_binary() {
        let binary: Vec<u8> = (0..100).map(|i| (i % 256) as u8).collect();
        // Invalid UTF-8 at certain points
        let mut binary = binary;
        binary[0] = 0x80; // Invalid UTF-8 start byte
        assert_eq!(
            detect_mime(&binary),
            MimeType::Other("application/octet-stream".into())
        );
    }
}
