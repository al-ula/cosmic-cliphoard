// SPDX-License-Identifier: MPL-2.0

//! CLI argument parsing using clap.

use clap::{Parser, Subcommand};
use cliphoard_schema::MimeType;

/// Clipboard manager for COSMIC desktop.
#[derive(Parser, Debug)]
#[command(name = "cliphoard", version, about)]
pub struct Cli {
    /// Subcommand to run.
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Available subcommands.
#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Show the overlay (default when no command is given).
    Toggle,

    /// Run as a COSMIC panel applet.
    Applet,

    /// Run as a system tray icon.
    Tray,

    /// Run the D-Bus daemon.
    Daemon,

    /// Decode clipboard content from stdin.
    Decode {
        /// MIME type of the input. Auto-detected if not specified.
        #[arg(short, long)]
        mime: Option<String>,

        /// Output as JSON instead of raw content.
        #[arg(short, long)]
        json: bool,
    },

    /// List clipboard entries.
    List {
        /// Show only the first N entries.
        #[arg(short = 'n', long)]
        limit: Option<usize>,

        /// Filter entries by search query.
        #[arg(short, long)]
        query: Option<String>,
    },

    /// Paste an entry to the clipboard.
    Paste {
        /// Entry ID to paste.
        id: u64,
    },

    /// Delete an entry from history.
    Delete {
        /// Entry ID to delete.
        id: u64,
    },

    /// Pin an entry to prevent eviction.
    Pin {
        /// Entry ID to pin.
        id: u64,
    },

    /// Unpin an entry.
    Unpin {
        /// Entry ID to unpin.
        id: u64,
    },

    /// Clear all non-pinned entries.
    Clear,

    /// Open the settings view.
    Settings,
}

impl Command {
    /// Parse MIME type string.
    pub fn parse_mime(s: &str) -> Option<MimeType> {
        Some(MimeType::parse(s))
    }
}
