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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{ClipboardEntry, ClipboardHistory, MimeType, SensitiveInfo};

    #[test]
    fn oxicode_roundtrip_entry() {
        let entry = ClipboardEntry::new(
            1,
            MimeType::TextPlain,
            b"hello".to_vec(),
            SensitiveInfo::normal(),
        );
        let bytes = OxiCodeCodec::serialize(&entry).unwrap();
        let decoded: ClipboardEntry = OxiCodeCodec::deserialize(&bytes).unwrap();
        assert_eq!(decoded.id, entry.id);
        assert_eq!(decoded.as_text(), Some("hello"));
    }

    #[test]
    fn oxicode_roundtrip_history() {
        let mut history = ClipboardHistory::new(10, 10, 1024 * 1024);
        history.push(
            MimeType::TextPlain,
            b"one".to_vec(),
            SensitiveInfo::normal(),
        );
        history.push(
            MimeType::TextHtml,
            b"<b>two</b>".to_vec(),
            SensitiveInfo::normal(),
        );

        let bytes = OxiCodeCodec::serialize(&history).unwrap();
        let decoded: ClipboardHistory = OxiCodeCodec::deserialize(&bytes).unwrap();
        assert_eq!(decoded.len(), 2);
    }
}
