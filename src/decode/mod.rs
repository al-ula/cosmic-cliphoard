// SPDX-License-Identifier: MPL-2.0

#[allow(clippy::module_inception)]
pub mod decode;
pub mod detect;

pub use decode::decode_stdin;
pub use detect::detect_mime;
