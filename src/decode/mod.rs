// SPDX-License-Identifier: MPL-2.0

pub mod decode;
pub mod detect;

pub use decode::{DecodeError, decode_content, decode_stdin};
pub use detect::{DetectionError, detect_mime};
