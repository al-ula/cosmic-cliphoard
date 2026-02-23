// SPDX-License-Identifier: MPL-2.0

//! Dual serialization codec for IPC and storage.
//!
//! Three usage modes:
//! - **Direct types** — use [`ClipboardEntry`] and [`ClipboardHistory`] as plain Rust types
//!   in the same process (no serialization overhead).
//! - **Bincode** — [`BincodeCodec`] for efficient binary IPC between separate processes.
//! - **JSON** — [`JsonCodec`] for human-readable storage and debugging.

use serde::{Serialize, de::DeserializeOwned};

/// Serialization/deserialization codec trait.
pub trait Codec {
    type Error: std::error::Error + Send + Sync + 'static;

    fn serialize<T: Serialize + bincode::Encode>(value: &T) -> Result<Vec<u8>, Self::Error>;
    fn deserialize<T: DeserializeOwned + bincode::Decode<()>>(bytes: &[u8]) -> Result<T, Self::Error>;
}

/// Efficient binary codec for IPC between processes.
pub struct BincodeCodec;

#[derive(Debug, thiserror::Error)]
pub enum BincodeError {
    #[error("bincode encode: {0}")]
    Encode(#[from] bincode::error::EncodeError),
    #[error("bincode decode: {0}")]
    Decode(#[from] bincode::error::DecodeError),
}

impl Codec for BincodeCodec {
    type Error = BincodeError;

    fn serialize<T: Serialize + bincode::Encode>(value: &T) -> Result<Vec<u8>, Self::Error> {
        Ok(bincode::encode_to_vec(value, bincode::config::standard())?)
    }

    fn deserialize<T: DeserializeOwned + bincode::Decode<()>>(bytes: &[u8]) -> Result<T, Self::Error> {
        let (value, _) = bincode::decode_from_slice(bytes, bincode::config::standard())?;
        Ok(value)
    }
}

/// Human-readable JSON codec for storage and debugging.
pub struct JsonCodec;

#[derive(Debug, thiserror::Error)]
#[error("json: {0}")]
pub struct JsonError(#[from] serde_json::Error);

impl Codec for JsonCodec {
    type Error = JsonError;

    fn serialize<T: Serialize + bincode::Encode>(value: &T) -> Result<Vec<u8>, Self::Error> {
        Ok(serde_json::to_vec_pretty(value)?)
    }

    fn deserialize<T: DeserializeOwned + bincode::Decode<()>>(bytes: &[u8]) -> Result<T, Self::Error> {
        Ok(serde_json::from_slice(bytes)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ClipboardEntry, ClipboardHistory, MimeType};

    #[test]
    fn bincode_roundtrip_entry() {
        let entry = ClipboardEntry::new(1, MimeType::TextPlain, b"hello".to_vec());
        let bytes = BincodeCodec::serialize(&entry).unwrap();
        let decoded: ClipboardEntry = BincodeCodec::deserialize(&bytes).unwrap();
        assert_eq!(decoded.id, entry.id);
        assert_eq!(decoded.as_text(), Some("hello"));
    }

    #[test]
    fn json_roundtrip_entry() {
        let entry = ClipboardEntry::new(1, MimeType::ImagePng, vec![0x89, 0x50]);
        let bytes = JsonCodec::serialize(&entry).unwrap();
        // Verify it's valid JSON text.
        let json_str = std::str::from_utf8(&bytes).unwrap();
        assert!(json_str.contains("\"mime\""));
        let decoded: ClipboardEntry = JsonCodec::deserialize(&bytes).unwrap();
        assert_eq!(decoded.id, entry.id);
        assert_eq!(decoded.mime, MimeType::ImagePng);
    }

    #[test]
    fn bincode_roundtrip_history() {
        let mut history = ClipboardHistory::new(10);
        history.push(MimeType::TextPlain, b"one".to_vec());
        history.push(MimeType::TextHtml, b"<b>two</b>".to_vec());

        let bytes = BincodeCodec::serialize(&history).unwrap();
        let decoded: ClipboardHistory = BincodeCodec::deserialize(&bytes).unwrap();
        assert_eq!(decoded.len(), 2);
    }

    #[test]
    fn json_roundtrip_history() {
        let mut history = ClipboardHistory::new(10);
        history.push(MimeType::TextPlain, b"test".to_vec());

        let bytes = JsonCodec::serialize(&history).unwrap();
        let decoded: ClipboardHistory = JsonCodec::deserialize(&bytes).unwrap();
        assert_eq!(decoded.len(), 1);
    }
}
