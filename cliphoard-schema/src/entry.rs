// SPDX-License-Identifier: MPL-2.0

//! Clipboard entry types.

use crate::MimeType;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Unique identifier for a clipboard entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(bincode::Encode, bincode::Decode)]
pub struct EntryId(pub u64);

impl std::fmt::Display for EntryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A single clipboard entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(bincode::Encode, bincode::Decode)]
pub struct ClipboardEntry {
    pub id: EntryId,
    pub mime: MimeType,
    /// Raw clipboard data bytes.
    pub data: Vec<u8>,
    /// Unix timestamp in milliseconds (for bincode compatibility with chrono).
    pub timestamp_ms: i64,
    /// Whether the user has pinned this entry to prevent eviction.
    pub pinned: bool,
}

impl ClipboardEntry {
    pub fn new(id: u64, mime: MimeType, data: Vec<u8>) -> Self {
        Self {
            id: EntryId(id),
            mime,
            data,
            timestamp_ms: Utc::now().timestamp_millis(),
            pinned: false,
        }
    }

    /// Get the timestamp as a chrono DateTime.
    pub fn timestamp(&self) -> DateTime<Utc> {
        DateTime::from_timestamp_millis(self.timestamp_ms)
            .unwrap_or_default()
    }

    /// For text entries, decode data as UTF-8.
    pub fn as_text(&self) -> Option<&str> {
        if self.mime.is_text() {
            std::str::from_utf8(&self.data).ok()
        } else {
            None
        }
    }

    /// A short preview string for display in UI lists.
    pub fn preview(&self, max_len: usize) -> String {
        if let Some(text) = self.as_text() {
            let trimmed = text.trim();
            if trimmed.len() <= max_len {
                trimmed.to_owned()
            } else {
                let mut s = trimmed[..max_len].to_owned();
                s.push_str("…");
                s
            }
        } else if self.mime.is_image() {
            format!("[Image: {}]", self.mime)
        } else {
            format!("[{}: {} bytes]", self.mime, self.data.len())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_entry_preview() {
        let entry = ClipboardEntry::new(1, MimeType::TextPlain, b"hello world".to_vec());
        assert_eq!(entry.preview(100), "hello world");
        assert_eq!(entry.as_text(), Some("hello world"));
    }

    #[test]
    fn image_entry_preview() {
        let entry = ClipboardEntry::new(2, MimeType::ImagePng, vec![0x89, 0x50, 0x4E, 0x47]);
        assert_eq!(entry.preview(100), "[Image: image/png]");
        assert!(entry.as_text().is_none());
    }

    #[test]
    fn long_text_truncation() {
        let long = "a".repeat(200);
        let entry = ClipboardEntry::new(3, MimeType::TextPlain, long.into_bytes());
        let preview = entry.preview(50);
        assert!(preview.len() <= 54); // 50 + "…" (3 bytes UTF-8)
        assert!(preview.ends_with("…"));
    }
}
