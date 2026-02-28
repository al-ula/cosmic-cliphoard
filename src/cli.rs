// SPDX-License-Identifier: MPL-2.0

use std::path::PathBuf;

use crate::schema::MimeType;
use clap::{Parser, Subcommand};
#[derive(Parser, Debug)]
#[command(name = "cliphoard", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}
#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Toggle the clipboard overlay
    Toggle,

    /// Run as a COSMIC panel applet
    Applet,

    /// Run the system tray icon
    Tray,

    /// Run the clipboard daemon
    Daemon,

    /// Decode clipboard content from stdin
    Decode {
        /// MIME type of the input
        #[arg(short, long)]
        mime: Option<String>,

        /// Output as JSON
        #[arg(short, long)]
        json: bool,
    },

    /// List clipboard entries
    List {
        /// Maximum number of entries to show
        #[arg(short = 'n', long)]
        limit: Option<usize>,

        /// Search query to filter entries
        #[arg(short, long)]
        query: Option<String>,
    },

    /// Paste a clipboard entry by ID
    Paste {
        id: u64,
    },

    /// Delete a clipboard entry by ID
    Delete {
        id: u64,
    },

    /// Pin a clipboard entry
    Pin {
        id: u64,
    },

    /// Unpin a clipboard entry
    Unpin {
        id: u64,
    },

    /// Clear all clipboard history
    Clear,

    /// Open the settings overlay
    Settings,

    /// Generate the systemd user service file
    GenerateService {
        /// Directory to write the service file into
        /// [default: ~/.config/systemd/user]
        #[arg(short, long)]
        path: Option<PathBuf>,
    },
}

impl Command {
    pub fn parse_mime(s: &str) -> Option<MimeType> {
        Some(MimeType::parse(s))
    }
}
