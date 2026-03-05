// SPDX-License-Identifier: MPL-2.0

//! Shared types and D-Bus interface definitions for the cliphoard clipboard manager.
//!
//! Uses **oxicode** for efficient binary serialization (IPC and history storage)
//! and **JSON** for human-editable config files.

pub mod codec;
pub mod config;
pub mod dbus;
pub mod entry;
pub mod history;
pub mod mime;
pub mod sensitive;

pub use codec::{Codec, OxiCodeCodec, OxiCodeError};
pub use config::{ClipboardConfig, KeyBinding, KeybindingsConfig, UIConfig};
pub use dbus::ClipboardManagerProxy;
pub use entry::{ClipboardEntry, EntryId};
pub use history::{ClipboardHistory, PinResult};
pub use mime::MimeType;
pub use sensitive::{DetectionConfig, SensitiveInfo};

/// The base application ID.
pub const APP_ID: &str = "com.github.al_ula.Cliphoard";

/// The applet application ID.
pub const APPLET_ID: &str = "com.github.al_ula.Cliphoard.Applet";

/// The D-Bus well-known name for the cliphoard daemon service.
pub const DBUS_NAME: &str = APP_ID;

/// The D-Bus object path for the clipboard manager.
pub const DBUS_PATH: &str = "/com/github/al_ula/Cliphoard";

/// Default maximum number of unpinned entries in clipboard history.
pub const DEFAULT_MAX_UNPINNED: usize = 500;

/// Default maximum number of pinned entries in clipboard history.
pub const DEFAULT_MAX_PINNED: usize = 50;

/// Default maximum size in bytes for a single clipboard entry's data.
pub const DEFAULT_MAX_ENTRY_SIZE: usize = 10 * 1024 * 1024; // 10 MiB
