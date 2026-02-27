// SPDX-License-Identifier: MPL-2.0

//! D-Bus daemon and clipboard watcher for cliphoard.
//!
//! Provides:
//! - `watcher`: Wayland clipboard monitoring via wl-paste
//! - `storage`: JSON-based history persistence
//! - `service`: D-Bus service implementation

pub mod service;
pub mod storage;
pub mod watcher;

pub use service::ClipboardManagerService;
pub use storage::Storage;
pub use watcher::ClipboardWatcher;
