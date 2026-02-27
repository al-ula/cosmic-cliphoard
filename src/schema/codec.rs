// SPDX-License-Identifier: MPL-2.0

use serde::{Serialize, de::DeserializeOwned};

pub trait Codec {
    type Error: std::error::Error + Send + Sync + 'static;

    fn serialize<T: Serialize + oxicode::Encode>(value: &T) -> Result<Vec<u8>, Self::Error>;
    fn deserialize<T: DeserializeOwned + oxicode::Decode<()>>(
        bytes: &[u8],
    ) -> Result<T, Self::Error>;
}

pub struct OxiCodeCodec;

#[derive(Debug, thiserror::Error)]
pub enum OxiCodeError {
    #[error("oxicode encode: {0}")]
    Encode(#[from] oxicode::error::Error),
}

impl Codec for OxiCodeCodec {
    type Error = OxiCodeError;

    fn serialize<T: Serialize + oxicode::Encode>(value: &T) -> Result<Vec<u8>, Self::Error> {
        Ok(oxicode::encode_to_vec(value)?)
    }

    fn deserialize<T: DeserializeOwned + oxicode::Decode<()>>(
        bytes: &[u8],
    ) -> Result<T, Self::Error> {
        let (value, _) = oxicode::decode_from_slice(bytes)?;
        Ok(value)
    }
}

pub struct JsonCodec;

#[derive(Debug, thiserror::Error)]
#[error("json: {0}")]
pub struct JsonError(#[from] serde_json::Error);

impl Codec for JsonCodec {
    type Error = JsonError;

    fn serialize<T: Serialize + oxicode::Encode>(value: &T) -> Result<Vec<u8>, Self::Error> {
        Ok(serde_json::to_vec_pretty(value)?)
    }

    fn deserialize<T: DeserializeOwned + oxicode::Decode<()>>(
        bytes: &[u8],
    ) -> Result<T, Self::Error> {
        Ok(serde_json::from_slice(bytes)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{ClipboardEntry, ClipboardHistory, MimeType};

    #[test]
    fn oxicode_roundtrip_entry() {
        let entry = ClipboardEntry::new(1, MimeType::TextPlain, b"hello".to_vec());
        let bytes = OxiCodeCodec::serialize(&entry).unwrap();
        let decoded: ClipboardEntry = OxiCodeCodec::deserialize(&bytes).unwrap();
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
    fn oxicode_roundtrip_history() {
        let mut history = ClipboardHistory::new(10, 10, 1024 * 1024);
        history.push(MimeType::TextPlain, b"one".to_vec());
        history.push(MimeType::TextHtml, b"<b>two</b>".to_vec());

        let bytes = OxiCodeCodec::serialize(&history).unwrap();
        let decoded: ClipboardHistory = OxiCodeCodec::deserialize(&bytes).unwrap();
        assert_eq!(decoded.len(), 2);
    }

    #[test]
    fn json_roundtrip_history() {
        let mut history = ClipboardHistory::new(10, 10, 1024 * 1024);
        history.push(MimeType::TextPlain, b"test".to_vec());

        let bytes = JsonCodec::serialize(&history).unwrap();
        let decoded: ClipboardHistory = JsonCodec::deserialize(&bytes).unwrap();
        assert_eq!(decoded.len(), 1);
    }
}
