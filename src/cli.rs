// SPDX-License-Identifier: MPL-2.0

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
    Toggle,

    Applet,

    Tray,

    Daemon,

    Decode {
        #[arg(short, long)]
        mime: Option<String>,

        #[arg(short, long)]
        json: bool,
    },

    List {
        #[arg(short = 'n', long)]
        limit: Option<usize>,

        #[arg(short, long)]
        query: Option<String>,
    },

    Paste {
        id: u64,
    },

    Delete {
        id: u64,
    },

    Pin {
        id: u64,
    },

    Unpin {
        id: u64,
    },

    Clear,

    Settings,
}

impl Command {
    pub fn parse_mime(s: &str) -> Option<MimeType> {
        Some(MimeType::parse(s))
    }
}
