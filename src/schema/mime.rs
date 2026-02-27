// SPDX-License-Identifier: MPL-2.0

//! MIME type classification for clipboard content.

use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, oxicode::Encode, oxicode::Decode,
)]
pub enum MimeType {
    TextPlain,
    TextHtml,
    TextUri,
    ImagePng,
    ImageJpeg,
    ImageSvg,
    ImageBmp,
    Other(String),
}

impl MimeType {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "text/plain" | "text/plain;charset=utf-8" | "utf8_string" | "string" => Self::TextPlain,
            "text/html" => Self::TextHtml,
            "text/uri-list" => Self::TextUri,
            "image/png" => Self::ImagePng,
            "image/jpeg" | "image/jpg" => Self::ImageJpeg,
            "image/svg+xml" => Self::ImageSvg,
            "image/bmp" => Self::ImageBmp,
            other => Self::Other(other.to_owned()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::TextPlain => "text/plain",
            Self::TextHtml => "text/html",
            Self::TextUri => "text/uri-list",
            Self::ImagePng => "image/png",
            Self::ImageJpeg => "image/jpeg",
            Self::ImageSvg => "image/svg+xml",
            Self::ImageBmp => "image/bmp",
            Self::Other(s) => s.as_str(),
        }
    }

    pub fn is_text(&self) -> bool {
        matches!(self, Self::TextPlain | Self::TextHtml | Self::TextUri)
    }

    pub fn is_image(&self) -> bool {
        matches!(
            self,
            Self::ImagePng | Self::ImageJpeg | Self::ImageSvg | Self::ImageBmp
        )
    }
}

impl std::fmt::Display for MimeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_common_types() {
        assert_eq!(MimeType::parse("text/plain"), MimeType::TextPlain);
        assert_eq!(
            MimeType::parse("text/plain;charset=utf-8"),
            MimeType::TextPlain
        );
        assert_eq!(MimeType::parse("UTF8_STRING"), MimeType::TextPlain);
        assert_eq!(MimeType::parse("image/png"), MimeType::ImagePng);
        assert_eq!(MimeType::parse("image/jpeg"), MimeType::ImageJpeg);
        assert_eq!(MimeType::parse("image/jpg"), MimeType::ImageJpeg);
        assert_eq!(MimeType::parse("text/html"), MimeType::TextHtml);
        assert_eq!(MimeType::parse("text/uri-list"), MimeType::TextUri);
    }

    #[test]
    fn parse_unknown() {
        assert_eq!(
            MimeType::parse("application/pdf"),
            MimeType::Other("application/pdf".into())
        );
    }
}
