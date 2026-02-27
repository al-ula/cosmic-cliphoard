// SPDX-License-Identifier: MPL-2.0

//! JSON-based persistence for clipboard history.

use cliphoard_schema::{ClipboardConfig, ClipboardHistory, JsonCodec, Codec, JsonError};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] JsonError),

    #[error("Failed to get data directory")]
    DataDir,
}

pub struct Storage {
    path: PathBuf,
    config_path: PathBuf,
}

impl Storage {
    pub fn new() -> Result<Self, StorageError> {
        let data_dir = dirs::data_dir().ok_or(StorageError::DataDir)?;
        let cliphoard_dir = data_dir.join("cliphoard");

        std::fs::create_dir_all(&cliphoard_dir)?;

        let path = cliphoard_dir.join("history.json");

        let config_dir = dirs::config_dir().ok_or(StorageError::DataDir)?;
        let config_cliphoard_dir = config_dir.join("cliphoard");
        std::fs::create_dir_all(&config_cliphoard_dir)?;
        let config_path = config_cliphoard_dir.join("config.json");

        debug!(?path, ?config_path, "Initialized storage");

        Ok(Self { path, config_path })
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    pub fn load(&self) -> Result<ClipboardHistory, StorageError> {
        if !self.path.exists() {
            info!("No existing history file, starting fresh");
            return Ok(ClipboardHistory::default());
        }

        debug!(path = %self.path.display(), "Loading history from disk");
        
        let bytes = std::fs::read(&self.path)?;
        
        if bytes.is_empty() {
            return Ok(ClipboardHistory::default());
        }

        match JsonCodec::deserialize::<ClipboardHistory>(&bytes) {
            Ok(history) => {
                info!(len = history.len(), "Loaded history from disk");
                Ok(history)
            }
            Err(e) => {
                warn!(error = %e, "Failed to parse history file, starting fresh");
                Ok(ClipboardHistory::default())
            }
        }
    }

    pub fn save(&self, history: &ClipboardHistory) -> Result<(), StorageError> {
        debug!(path = %self.path.display(), len = history.len(), "Saving history to disk");
        
        let bytes = JsonCodec::serialize(history)?;
        
        // Write to temp file first, then rename for atomicity
        let temp_path = self.path.with_extension("json.tmp");
        std::fs::write(&temp_path, &bytes)?;
        std::fs::rename(&temp_path, &self.path)?;
        
        info!(bytes = bytes.len(), "History saved");
        Ok(())
    }

    pub async fn save_async(&self, history: Arc<RwLock<ClipboardHistory>>) -> Result<(), StorageError> {
        let hist = history.read().await;
        self.save(&hist)
    }

    pub fn load_config(&self) -> ClipboardConfig {
        if !self.config_path.exists() {
            debug!("No config file found, using defaults");
            return ClipboardConfig::default();
        }

        match std::fs::read(&self.config_path) {
            Ok(bytes) if !bytes.is_empty() => {
                match serde_json::from_slice::<ClipboardConfig>(&bytes) {
                    Ok(config) => {
                        info!(?config, "Loaded config from disk");
                        config
                    }
                    Err(e) => {
                        warn!(error = %e, "Failed to parse config file, using defaults");
                        ClipboardConfig::default()
                    }
                }
            }
            _ => ClipboardConfig::default(),
        }
    }

    pub fn save_config(&self, config: &ClipboardConfig) -> Result<(), StorageError> {
        debug!(?config, path = %self.config_path.display(), "Saving config to disk");

        let bytes = serde_json::to_vec_pretty(config).map_err(|e| StorageError::Json(JsonError::from(e)))?;
        let temp_path = self.config_path.with_extension("json.tmp");
        std::fs::write(&temp_path, &bytes)?;
        std::fs::rename(&temp_path, &self.config_path)?;

        info!("Config saved");
        Ok(())
    }

    pub fn clear(&self) -> Result<(), StorageError> {
        if self.path.exists() {
            std::fs::remove_file(&self.path)?;
            info!("History file removed");
        }
        Ok(())
    }
}

impl Default for Storage {
    fn default() -> Self {
        Self::new().expect("Failed to initialize storage")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cliphoard_schema::MimeType;
    use tempfile::tempdir;

    #[test]
    fn save_and_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("history.json");
        
        let mut history = ClipboardHistory::new(10, 10, 1024 * 1024);
        history.push(MimeType::TextPlain, b"test data".to_vec());
        
        // Create storage with custom path for testing
        let storage = Storage {
            path: path.clone(),
            config_path: dir.path().join("config.json"),
        };
        
        storage.save(&history).unwrap();
        let loaded = storage.load().unwrap();
        
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.iter().next().unwrap().as_text(), Some("test data"));
    }
}
