// SPDX-License-Identifier: MPL-2.0

//! Wayland clipboard watcher using wayland-clipboard-listener.

use cliphoard_decode::detect_mime;
use cliphoard_schema::ClipboardHistory;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};
use wayland_clipboard_listener::{WlClipboardPasteStream, WlListenType};

#[derive(Debug, thiserror::Error)]
pub enum WatcherError {
    #[error("Failed to create clipboard listener: {0}")]
    ListenerCreate(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct ClipboardWatcher {
    history: Arc<RwLock<ClipboardHistory>>,
}

impl ClipboardWatcher {
    pub fn new(history: Arc<RwLock<ClipboardHistory>>) -> Self {
        Self { history }
    }

    pub async fn start(&self) -> Result<(), WatcherError> {
        info!("Starting clipboard watcher");

        let history = Arc::clone(&self.history);
        
        tokio::spawn(async move {
            let result = Self::watch_loop(history).await;
            if let Err(e) = result {
                error!("Clipboard watcher error: {}", e);
            }
        });

        Ok(())
    }

    async fn watch_loop(history: Arc<RwLock<ClipboardHistory>>) -> Result<(), WatcherError> {
        let mut stream = WlClipboardPasteStream::init(WlListenType::ListenOnCopy)
            .map_err(|e| WatcherError::ListenerCreate(e.to_string()))?;

        info!("Clipboard listener created, waiting for changes...");

        for msg in stream.paste_stream().flatten() {
            // ClipBoardListenContext has:
            // - mime_type: String
            // - context: Vec<u8> (the actual data)
            let ctx = msg.context;
            let data = ctx.context;
            let mime_from_wl = ctx.mime_type;
            
            if data.is_empty() {
                debug!("Received empty clipboard data, skipping");
                continue;
            }

            let mime = detect_mime(&data);
            debug!(
                wl_mime = %mime_from_wl,
                detected_mime = %mime,
                len = data.len(),
                "New clipboard content detected"
            );

            {
                let hist = history.read().await;
                let is_duplicate = hist.iter().any(|e| e.data == data);
                if is_duplicate {
                    debug!("Duplicate clipboard content, skipping");
                    continue;
                }
            }

            {
                let mut hist = history.write().await;
                match hist.push(mime.clone(), data) {
                    Some(id) => {
                        info!(%id, %mime, "Added new clipboard entry");
                    }
                    None => {
                        debug!(%mime, "Entry rejected (exceeds max entry size)");
                    }
                }
            }
        }

        Ok(())
    }
}
