// SPDX-License-Identifier: MPL-2.0

//! MIME detection and content decoding for cliphoard.
//!
//! Provides magic-byte based MIME detection without external dependencies,
//! and content decoding utilities for various clipboard formats.

pub mod decode;
pub mod detect;

pub use decode::{decode_content, decode_stdin, DecodeError};
pub use detect::{detect_mime, DetectionError};

/// Re-export from cliphoard-schema for convenience.
pub use cliphoard_schema::MimeType;
