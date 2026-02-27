// SPDX-License-Identifier: MPL-2.0

//! CLI command implementations.

use crate::cli::Command;
use cliphoard_schema::{ClipboardManagerProxy, OxiCodeCodec, Codec, ClipboardEntry};
use std::error::Error;
use tracing::{debug, info};
use zbus::Connection;

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("D-Bus error: {0}")]
    Zbus(#[from] zbus::Error),

    #[error("D-Bus method error: {0}")]
    Fdo(#[from] zbus::fdo::Error),

    #[error("Codec error: {0}")]
    Codec(#[from] cliphoard_schema::OxiCodeError),

    #[error("Entry not found: {0}")]
    NotFound(u64),

    #[error("Daemon not running. Start with `cliphoard daemon`")]
    DaemonNotRunning,
}

async fn get_proxy() -> Result<ClipboardManagerProxy<'static>, CommandError> {
    let connection = Connection::session().await?;
    let proxy = ClipboardManagerProxy::new(&connection).await?;
    Ok(proxy)
}

pub async fn run_daemon() -> Result<(), Box<dyn Error + Send + Sync>> {
    #[cfg(debug_assertions)]
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    
    #[cfg(not(debug_assertions))]
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    cliphoard_daemon::service::run_daemon().await?;
    Ok(())
}

pub fn run_decode(mime: Option<String>, json: bool) {
    let mime = mime.map(|s| cliphoard_schema::MimeType::parse(&s));
    
    if let Err(e) = cliphoard_decode::decode_stdin(mime, json) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

pub async fn run(cmd: Command) {
    #[cfg(debug_assertions)]
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    
    #[cfg(not(debug_assertions))]
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    if let Err(e) = run_command(cmd).await {
        eprintln!("Error: {}", e);
        let mut source = std::error::Error::source(&*e);
        while let Some(s) = source {
            eprintln!("  caused by: {}", s);
            source = std::error::Error::source(s);
        }
        std::process::exit(1);
    }
}

async fn run_command(cmd: Command) -> Result<(), Box<dyn Error + Send + Sync>> {
    match cmd {
        Command::List { limit, query } => {
            list_entries(limit, query).await?;
        }
        Command::Paste { id } => {
            paste_entry(id).await?;
        }
        Command::Delete { id } => {
            delete_entry(id).await?;
        }
        Command::Pin { id } => {
            pin_entry(id).await?;
        }
        Command::Unpin { id } => {
            unpin_entry(id).await?;
        }
        Command::Clear => {
            clear_history().await?;
        }
        _ => unreachable!(),
    }
    Ok(())
}

async fn list_entries(limit: Option<usize>, query: Option<String>) -> Result<(), CommandError> {
    debug!(?limit, ?query, "Listing entries");
    
    let proxy = get_proxy().await?;
    
    let bytes = if let Some(q) = query {
        proxy.search(&q).await?
    } else {
        proxy.list_entries().await?
    };

    let entries: Vec<ClipboardEntry> = OxiCodeCodec::deserialize(&bytes)?;
    
    let entries: Vec<_> = if let Some(n) = limit {
        entries.into_iter().take(n).collect()
    } else {
        entries
    };

    if entries.is_empty() {
        println!("No entries found.");
        return Ok(());
    }

    println!("{:<8} {:<6} {:<10} {}", "ID", "Pinned", "MIME", "Preview");
    println!("{}", "-".repeat(60));
    
    for entry in entries {
        let pinned = if entry.pinned { "yes" } else { "no" };
        println!(
            "{:<8} {:<6} {:<10} {}",
            entry.id.0,
            pinned,
            entry.mime.as_str().split('/').next_back().unwrap_or("?"),
            entry.preview(50)
        );
    }

    Ok(())
}

async fn paste_entry(id: u64) -> Result<(), CommandError> {
    debug!(id, "Pasting entry");
    
    let proxy = get_proxy().await?;
    let success = proxy.paste_entry(id).await?;
    
    if success {
        info!(id, "Entry pasted to clipboard");
        println!("Entry {} pasted to clipboard.", id);
    } else {
        return Err(CommandError::NotFound(id));
    }
    
    Ok(())
}

async fn delete_entry(id: u64) -> Result<(), CommandError> {
    debug!(id, "Deleting entry");
    
    let proxy = get_proxy().await?;
    let removed = proxy.delete_entry(id).await?;
    
    if removed {
        info!(id, "Entry deleted");
        println!("Entry {} deleted.", id);
    } else {
        return Err(CommandError::NotFound(id));
    }
    
    Ok(())
}

async fn pin_entry(id: u64) -> Result<(), CommandError> {
    debug!(id, "Pinning entry");
    
    let proxy = get_proxy().await?;
    let pinned = proxy.pin_entry(id).await?;
    
    if pinned {
        info!(id, "Entry pinned");
        println!("Entry {} pinned.", id);
    } else {
        return Err(CommandError::NotFound(id));
    }
    
    Ok(())
}

async fn unpin_entry(id: u64) -> Result<(), CommandError> {
    debug!(id, "Unpinning entry");
    
    let proxy = get_proxy().await?;
    let unpinned = proxy.unpin_entry(id).await?;
    
    if unpinned {
        info!(id, "Entry unpinned");
        println!("Entry {} unpinned.", id);
    } else {
        return Err(CommandError::NotFound(id));
    }
    
    Ok(())
}

async fn clear_history() -> Result<(), CommandError> {
    debug!("Clearing history");
    
    let proxy = get_proxy().await?;
    proxy.clear().await?;
    
    info!("History cleared");
    println!("History cleared.");
    
    Ok(())
}
