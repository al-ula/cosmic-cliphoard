// SPDX-License-Identifier: MPL-2.0

//! Clipboard entry types.

use super::MimeType;
use super::sensitive::SensitiveInfo;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    oxicode::Encode,
    oxicode::Decode,
)]
pub struct EntryId(pub u64);

impl std::fmt::Display for EntryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, oxicode::Encode, oxicode::Decode)]
pub struct ClipboardEntry {
    pub id: EntryId,
    pub mime: MimeType,
    pub data: Vec<u8>,
    pub timestamp_ms: i64,
    pub pinned: bool,
    #[serde(default)]
    pub sensitive: SensitiveInfo,
}

impl ClipboardEntry {
    pub fn new(id: u64, mime: MimeType, data: Vec<u8>, sensitive: SensitiveInfo) -> Self {
        Self {
            id: EntryId(id),
            mime,
            data,
            timestamp_ms: Utc::now().timestamp_millis(),
            pinned: false,
            sensitive,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        if self.mime.is_text() {
            std::str::from_utf8(&self.data).ok()
        } else {
            None
        }
    }

    /// Returns a searchable string representation for any entry type.
    /// For text entries this is the text content itself.
    /// For non-text entries this includes MIME type and auto-extracted metadata
    /// (e.g. image dimensions, URI filenames, HTML titles).
    pub fn search_text(&self) -> String {
        if let Some(text) = self.as_text() {
            // For URI lists, append extracted filenames so users can search
            // by filename without needing the full path.
            if self.mime == MimeType::TextUri {
                let filenames = extract_uri_filenames(text);
                if filenames.is_empty() {
                    return text.to_owned();
                }
                return format!("{} {}", text, filenames.join(" "));
            }
            return text.to_owned();
        }

        let mut parts = Vec::new();

        // Always include the MIME type
        parts.push(self.mime.to_string());

        // Include broad category keywords
        if self.mime.is_image() {
            parts.push("image".into());
        }

        // Extract metadata from image headers
        if let Some(dims) = self.image_dimensions() {
            parts.push(format!("{}x{}", dims.0, dims.1));
        }

        // Human-readable size
        parts.push(human_size(self.data.len()));

        parts.join(" ")
    }

    /// Attempt to read image dimensions from headers without decoding the full image.
    fn image_dimensions(&self) -> Option<(u32, u32)> {
        let d = &self.data;
        match self.mime {
            MimeType::ImagePng if d.len() >= 24 => {
                let w = u32::from_be_bytes([d[16], d[17], d[18], d[19]]);
                let h = u32::from_be_bytes([d[20], d[21], d[22], d[23]]);
                Some((w, h))
            }
            MimeType::ImageJpeg => parse_jpeg_dimensions(d),
            MimeType::ImageBmp if d.len() >= 26 => {
                let w = u32::from_le_bytes([d[18], d[19], d[20], d[21]]);
                let h = u32::from_le_bytes([d[22], d[23], d[24], d[25]]);
                Some((w, h))
            }
            _ => None,
        }
    }

    pub fn preview(&self, max_len: usize) -> String {
        if let Some(text) = self.as_text() {
            // For URI entries, show filenames instead of raw URIs
            if self.mime == MimeType::TextUri {
                let names = extract_uri_filenames(text);
                if !names.is_empty() {
                    let joined = names.join(", ");
                    if joined.len() <= max_len {
                        return joined;
                    }
                    let truncated = match joined.char_indices().nth(max_len) {
                        Some((idx, _)) => &joined[..idx],
                        None => &joined,
                    };
                    let mut s = truncated.to_owned();
                    s.push('…');
                    return s;
                }
            }

            let collapsed: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
            if collapsed.len() <= max_len {
                collapsed
            } else {
                let truncated = match collapsed.char_indices().nth(max_len) {
                    Some((idx, _)) => &collapsed[..idx],
                    None => &collapsed,
                };
                let mut s = truncated.to_owned();
                s.push('…');
                s
            }
        } else if self.mime.is_image() {
            format!("[Image: {}]", self.mime)
        } else {
            format!("[{}: {} bytes]", self.mime, self.data.len())
        }
    }
}

/// Parse JPEG dimensions by scanning for SOF markers.
fn parse_jpeg_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    let mut i = 2; // skip SOI (FF D8)
    while i + 1 < data.len() {
        if data[i] != 0xFF {
            return None;
        }
        let marker = data[i + 1];
        if marker == 0xD9 {
            return None; // EOI
        }
        if i + 3 >= data.len() {
            return None;
        }
        let seg_len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
        // SOF0..SOF3 markers contain dimensions
        if (0xC0..=0xC3).contains(&marker) && i + 8 < data.len() {
            let h = u16::from_be_bytes([data[i + 5], data[i + 6]]) as u32;
            let w = u16::from_be_bytes([data[i + 7], data[i + 8]]) as u32;
            return Some((w, h));
        }
        i += 2 + seg_len;
    }
    None
}

/// Extract filenames from URI list lines (e.g. "file:///home/user/photo.png" → "photo.png").
fn extract_uri_filenames(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .filter_map(|uri| {
            let path = uri.trim().rsplit_once('/')?;
            let name = path.1;
            if name.is_empty() { None } else { Some(name) }
        })
        .collect()
}

fn human_size(bytes: usize) -> String {
    const KIB: usize = 1024;
    const MIB: usize = 1024 * KIB;
    if bytes >= MIB {
        format!("{:.1}MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1}KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{}B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normal_sensitive() -> SensitiveInfo {
        SensitiveInfo::normal()
    }

    #[test]
    fn text_entry_preview() {
        let entry = ClipboardEntry::new(
            1,
            MimeType::TextPlain,
            b"hello world".to_vec(),
            normal_sensitive(),
        );
        assert_eq!(entry.preview(100), "hello world");
        assert_eq!(entry.as_text(), Some("hello world"));
    }

    #[test]
    fn image_entry_preview() {
        let entry = ClipboardEntry::new(
            2,
            MimeType::ImagePng,
            vec![0x89, 0x50, 0x4E, 0x47],
            normal_sensitive(),
        );
        assert_eq!(entry.preview(100), "[Image: image/png]");
        assert!(entry.as_text().is_none());
    }

    #[test]
    fn long_text_truncation() {
        let long = "a".repeat(200);
        let entry = ClipboardEntry::new(
            3,
            MimeType::TextPlain,
            long.into_bytes(),
            normal_sensitive(),
        );
        let preview = entry.preview(50);
        assert!(preview.len() <= 54);
        assert!(preview.ends_with("…"));
    }

    #[test]
    fn uri_entry_preview_shows_filenames() {
        let uri = b"file:///home/user/doc.pdf\nfile:///tmp/image.png";
        let entry = ClipboardEntry::new(5, MimeType::TextUri, uri.to_vec(), normal_sensitive());
        assert_eq!(entry.preview(100), "doc.pdf, image.png");
    }

    #[test]
    fn uri_entry_preview_truncates() {
        let uri = b"file:///a/very_long_filename_one.txt\nfile:///b/very_long_filename_two.txt";
        let entry = ClipboardEntry::new(6, MimeType::TextUri, uri.to_vec(), normal_sensitive());
        let preview = entry.preview(30);
        assert!(preview.ends_with('…'));
        assert!(preview.len() <= 34);
    }

    #[test]
    fn search_text_for_text_entry() {
        let entry = ClipboardEntry::new(
            1,
            MimeType::TextPlain,
            b"hello world".to_vec(),
            normal_sensitive(),
        );
        assert_eq!(entry.search_text(), "hello world");
    }

    #[test]
    fn search_text_for_image_includes_mime_and_dimensions() {
        let mut png = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        png.extend_from_slice(&[0x00, 0x00, 0x00, 0x0D]);
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&100u32.to_be_bytes());
        png.extend_from_slice(&200u32.to_be_bytes());
        let entry = ClipboardEntry::new(2, MimeType::ImagePng, png, normal_sensitive());
        let st = entry.search_text();
        assert!(st.contains("image/png"), "should contain mime type");
        assert!(st.contains("image"), "should contain category keyword");
        assert!(st.contains("100x200"), "should contain dimensions");
    }

    #[test]
    fn search_text_for_unknown_binary() {
        let entry = ClipboardEntry::new(
            3,
            MimeType::Other("application/pdf".into()),
            vec![0; 2048],
            normal_sensitive(),
        );
        let st = entry.search_text();
        assert!(st.contains("application/pdf"));
        assert!(st.contains("KiB"));
    }

    #[test]
    fn human_size_formatting() {
        assert_eq!(human_size(500), "500B");
        assert_eq!(human_size(1024), "1.0KiB");
        assert_eq!(human_size(1024 * 1024), "1.0MiB");
    }

    #[test]
    fn search_text_uri_includes_filenames() {
        let uri = b"file:///home/user/Documents/report.pdf\nhttps://example.com/photo.png";
        let entry = ClipboardEntry::new(4, MimeType::TextUri, uri.to_vec(), normal_sensitive());
        let st = entry.search_text();
        assert!(st.contains("report.pdf"), "should contain local filename");
        assert!(st.contains("photo.png"), "should contain remote filename");
    }

    #[test]
    fn extract_uri_filenames_skips_comments_and_empty() {
        let names = extract_uri_filenames("# comment\nfile:///a/b.txt\n\nhttps://x/c.jpg");
        assert_eq!(names, vec!["b.txt", "c.jpg"]);
    }

    #[test]
    fn extract_uri_filenames_trailing_slash() {
        let names = extract_uri_filenames("https://example.com/");
        assert!(names.is_empty());
    }
}
