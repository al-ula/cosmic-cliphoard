// SPDX-License-Identifier: MPL-2.0

use crate::schema::{
    ClipboardConfig, ClipboardHistory, Codec, DBUS_NAME, DBUS_PATH, EntryId, OxiCodeCodec,
    OxiCodeError, PinResult,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};
use zbus::{connection::Builder, interface, object_server::SignalEmitter};

use super::watcher::WatcherError;

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("D-Bus error: {0}")]
    Zbus(#[from] zbus::Error),

    #[error("Storage error: {0}")]
    Storage(#[from] super::storage::StorageError),

    #[error("Codec error: {0}")]
    Codec(#[from] OxiCodeError),

    #[error("Watcher error: {0}")]
    Watcher(#[from] WatcherError),

    #[error("Clipboard write error: {0}")]
    ClipboardWrite(#[from] super::clipboard_writer::ClipboardWriteError),
}

pub struct ClipboardManagerService {
    history: Arc<RwLock<ClipboardHistory>>,
    storage: Arc<super::storage::Storage>,
}

impl ClipboardManagerService {
    pub fn new(
        history: Arc<RwLock<ClipboardHistory>>,
        storage: Arc<super::storage::Storage>,
    ) -> Self {
        Self { history, storage }
    }

    async fn persist(&self) {
        let hist = self.history.read().await;
        if let Err(e) = self.storage.save(&hist) {
            error!("Failed to save history: {e}");
        }
    }
}

#[interface(name = "com.github.al_ula.Cliphoard.Manager")]
impl ClipboardManagerService {
    async fn list_entries(&self) -> zbus::fdo::Result<Vec<u8>> {
        debug!("list_entries called");
        let hist = self.history.read().await;
        let entries: Vec<_> = hist.iter().cloned().collect();
        OxiCodeCodec::serialize(&entries).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn get_entry(&self, id: u64) -> zbus::fdo::Result<Vec<u8>> {
        debug!(id, "get_entry called");
        let hist = self.history.read().await;
        match hist.get(EntryId(id)) {
            Some(entry) => {
                OxiCodeCodec::serialize(entry).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
            }
            None => Err(zbus::fdo::Error::UnknownObject(format!(
                "Entry {id} not found"
            ))),
        }
    }

    async fn search(&self, query: &str) -> zbus::fdo::Result<Vec<u8>> {
        debug!(query, "search called");
        let hist = self.history.read().await;
        let results: Vec<_> = hist.search(query).into_iter().cloned().collect();
        OxiCodeCodec::serialize(&results).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn delete_entry(&self, id: u64) -> zbus::fdo::Result<bool> {
        debug!(id, "delete_entry called");
        let removed = {
            let mut hist = self.history.write().await;
            hist.remove(EntryId(id))
        };
        if removed {
            self.persist().await;
            info!(id, "Entry deleted");
        }
        Ok(removed)
    }

    async fn pin_entry(&self, id: u64) -> zbus::fdo::Result<bool> {
        debug!(id, "pin_entry called");
        let result = {
            let mut hist = self.history.write().await;
            hist.pin(EntryId(id))
        };
        match result {
            PinResult::Pinned => {
                self.persist().await;
                Ok(true)
            }
            PinResult::NotFound => Ok(false),
            PinResult::LimitReached => {
                Err(zbus::fdo::Error::Failed("pin_limit_reached".to_string()))
            }
        }
    }

    async fn unpin_entry(&self, id: u64) -> zbus::fdo::Result<bool> {
        debug!(id, "unpin_entry called");
        let unpinned = {
            let mut hist = self.history.write().await;
            hist.unpin(EntryId(id))
        };
        if unpinned {
            self.persist().await;
        }
        Ok(unpinned)
    }

    async fn clear(&self) -> zbus::fdo::Result<()> {
        debug!("clear called");
        {
            let mut hist = self.history.write().await;
            hist.clear();
        }
        self.persist().await;
        info!("History cleared");
        Ok(())
    }

    async fn paste_entry(&self, id: u64) -> zbus::fdo::Result<bool> {
        debug!(id, "paste_entry called");

        let data = {
            let hist = self.history.read().await;
            match hist.get(EntryId(id)) {
                Some(entry) => (entry.mime.as_str().to_owned(), entry.data.clone()),
                None => return Ok(false),
            }
        };

        let mime = data.0;
        let payload = data.1;

        // The clipboard writer blocks until the source is cancelled (replaced by
        // another copy). We use a oneshot channel so the caller knows the clipboard
        // was set successfully, without waiting for the long-lived event loop.
        let (tx, rx) = tokio::sync::oneshot::channel();

        tokio::task::spawn_blocking(move || {
            match super::clipboard_writer::write_to_clipboard(&mime, payload, tx) {
                Ok(()) => {}
                Err(e) => {
                    error!("Clipboard write failed: {e}");
                }
            }
        });

        match rx.await {
            Ok(Ok(())) => {
                info!(id, "Pasted entry to clipboard");
                Ok(true)
            }
            Ok(Err(e)) => {
                error!("Clipboard write setup failed: {e}");
                Ok(false)
            }
            Err(_) => {
                error!("Clipboard writer dropped before confirming");
                Ok(false)
            }
        }
    }

    async fn update_config(
        &self,
        max_unpinned: u64,
        max_pinned: u64,
        max_entry_size: u64,
    ) -> zbus::fdo::Result<()> {
        debug!(
            max_unpinned,
            max_pinned, max_entry_size, "update_config called"
        );

        let max_unpinned = max_unpinned as usize;
        let max_pinned = max_pinned as usize;
        let max_entry_size = max_entry_size as usize;

        {
            let mut hist = self.history.write().await;
            hist.update_limits(max_unpinned, max_pinned, max_entry_size);
        }

        let config = ClipboardConfig {
            max_unpinned,
            max_pinned,
            max_entry_size,
        };

        if let Err(e) = self.storage.save_config(&config) {
            error!("Failed to save config: {e}");
            return Err(zbus::fdo::Error::Failed(e.to_string()));
        }

        info!("Config updated and saved");
        Ok(())
    }

    #[zbus(signal)]
    async fn entry_added(emitter: &SignalEmitter<'_>, id: u64) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn entry_removed(emitter: &SignalEmitter<'_>, id: u64) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn history_cleared(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

pub async fn run_daemon() -> Result<(), ServiceError> {
    info!("Starting cliphoard daemon");

    let storage = Arc::new(super::storage::Storage::new()?);
    let config = storage.load_config();
    let mut history_data = storage.load()?;
    history_data.update_limits(
        config.max_unpinned,
        config.max_pinned,
        config.max_entry_size,
    );
    let history = Arc::new(RwLock::new(history_data));

    let watcher = super::watcher::ClipboardWatcher::new(Arc::clone(&history));
    watcher.start().await?;

    let service = ClipboardManagerService::new(Arc::clone(&history), storage);

    let _connection = Builder::session()?
        .name(DBUS_NAME)?
        .serve_at(DBUS_PATH, service)?
        .build()
        .await?;

    info!("Daemon running on {}", DBUS_NAME);

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
    }
}
