// SPDX-License-Identifier: MPL-2.0

use crate::schema::MimeType;
use std::io::{self, Read, Write};
use thiserror::Error;
use tracing::debug;

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("UTF-8 decode error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

#[derive(Debug, Clone)]
pub enum DecodedContent {
    Text(String),
    Html(String),
    UriList(Vec<String>),
    Binary { mime: MimeType, data: Vec<u8> },
}

pub fn decode_content(mime: &MimeType, data: Vec<u8>) -> Result<DecodedContent, DecodeError> {
    debug!(?mime, len = data.len(), "Decoding content");

    match mime {
        MimeType::TextPlain => {
            let text = String::from_utf8(data)?;
            Ok(DecodedContent::Text(text))
        }

        MimeType::TextHtml => {
            let html = String::from_utf8(data)?;
            Ok(DecodedContent::Html(html))
        }

        MimeType::TextUri => {
            let text = String::from_utf8(data)?;
            let uris: Vec<String> = text
                .lines()
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .map(String::from)
                .collect();
            Ok(DecodedContent::UriList(uris))
        }

        MimeType::ImagePng
        | MimeType::ImageJpeg
        | MimeType::ImageSvg
        | MimeType::ImageBmp
        | MimeType::Other(_) => Ok(DecodedContent::Binary {
            mime: mime.clone(),
            data,
        }),
    }
}

pub fn decode_stdin(mime: Option<MimeType>, json: bool) -> Result<(), DecodeError> {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input)?;

    let detected_mime = mime.unwrap_or_else(|| super::detect::detect_mime(&input));

    let decoded = decode_content(&detected_mime, input)?;

    if json {
        output_json(&detected_mime, &decoded)?;
    } else {
        output_raw(&decoded)?;
    }

    Ok(())
}

fn output_json(mime: &MimeType, content: &DecodedContent) -> io::Result<()> {
    let stdout = io::stdout();
    let mut out = stdout.lock();

    match content {
        DecodedContent::Text(text) => {
            write!(out, "{{\"mime\":\"{mime}\",\"type\":\"text\",\"content\":")?;
            serde_json::to_writer(&mut out, text)?;
            writeln!(out, "}}")?;
        }
        DecodedContent::Html(html) => {
            write!(out, "{{\"mime\":\"{mime}\",\"type\":\"html\",\"content\":")?;
            serde_json::to_writer(&mut out, html)?;
            writeln!(out, "}}")?;
        }
        DecodedContent::UriList(uris) => {
            write!(out, "{{\"mime\":\"{mime}\",\"type\":\"uri\",\"uris\":")?;
            serde_json::to_writer(&mut out, uris)?;
            writeln!(out, "}}")?;
        }
        DecodedContent::Binary { mime, data } => {
            writeln!(
                out,
                "{{\"mime\":\"{mime}\",\"type\":\"binary\",\"size\":{}}}",
                data.len()
            )?;
        }
    }

    Ok(())
}

fn output_raw(content: &DecodedContent) -> io::Result<()> {
    let stdout = io::stdout();
    let mut out = stdout.lock();

    match content {
        DecodedContent::Text(text) | DecodedContent::Html(text) => {
            write!(out, "{text}")?;
        }
        DecodedContent::UriList(uris) => {
            for uri in uris {
                writeln!(out, "{uri}")?;
            }
        }
        DecodedContent::Binary { data, .. } => {
            out.write_all(data)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_text() {
        let data = b"Hello, world!".to_vec();
        let decoded = decode_content(&MimeType::TextPlain, data).unwrap();
        match decoded {
            DecodedContent::Text(text) => assert_eq!(text, "Hello, world!"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn decode_uri_list() {
        let data = b"# Comment\nfile:///home/user/doc.txt\nhttps://example.com\n".to_vec();
        let decoded = decode_content(&MimeType::TextUri, data).unwrap();
        match decoded {
            DecodedContent::UriList(uris) => {
                assert_eq!(uris.len(), 2);
                assert_eq!(uris[0], "file:///home/user/doc.txt");
                assert_eq!(uris[1], "https://example.com");
            }
            _ => panic!("Expected UriList variant"),
        }
    }

    #[test]
    fn decode_binary() {
        let data = vec![0x89, 0x50, 0x4E, 0x47];
        let decoded = decode_content(&MimeType::ImagePng, data.clone()).unwrap();
        match decoded {
            DecodedContent::Binary { mime, data: d } => {
                assert_eq!(mime, MimeType::ImagePng);
                assert_eq!(d, data);
            }
            _ => panic!("Expected Binary variant"),
        }
    }
}
