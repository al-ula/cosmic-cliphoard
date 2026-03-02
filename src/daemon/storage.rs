// SPDX-License-Identifier: MPL-2.0

use crate::schema::{ClipboardConfig, ClipboardHistory, Codec, OxiCodeCodec, OxiCodeError};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Codec(#[from] OxiCodeError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

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

    pub fn load(&self) -> Result<ClipboardHistory, StorageError> {
        // Migrate from old binary format if history.bin exists
        let bin_path = self.path.with_file_name("history.bin");
        if bin_path.exists() && !self.path.exists() {
            info!("Found history.bin, migrating to JSON format");
            if let Ok(history) = Self::load_binary(&bin_path)
                && let Ok(()) = Self::save_json(&self.path, &history)
            {
                let bak_path = bin_path.with_extension("bin.bak");
                if let Err(e) = std::fs::rename(&bin_path, &bak_path) {
                    warn!(error = %e, "Failed to rename history.bin to backup");
                } else {
                    info!(
                        "Migration complete: history.bin → history.json (backup at history.bin.bak)"
                    );
                }
                return Ok(history);
            }
            warn!("Migration failed, starting with empty history");
        }

        if !self.path.exists() {
            info!("No existing history file, starting fresh");
            return Ok(ClipboardHistory::default());
        }

        debug!(path = %self.path.display(), "Loading history from disk");

        let bytes = std::fs::read(&self.path)?;

        if bytes.is_empty() {
            return Ok(ClipboardHistory::default());
        }

        match serde_json::from_slice::<ClipboardHistory>(&bytes) {
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

    /// Load history from an oxicode binary file (used for migration).
    fn load_binary(path: &Path) -> Result<ClipboardHistory, StorageError> {
        let bytes = std::fs::read(path)?;
        if bytes.is_empty() {
            return Ok(ClipboardHistory::default());
        }
        Ok(OxiCodeCodec::deserialize::<ClipboardHistory>(&bytes)?)
    }

    /// Save history as pretty-printed JSON.
    fn save_json(path: &Path, history: &ClipboardHistory) -> Result<(), StorageError> {
        let bytes = serde_json::to_vec_pretty(history)?;
        let temp_path = path.with_extension("json.tmp");
        std::fs::write(&temp_path, &bytes)?;
        std::fs::rename(&temp_path, path)?;
        Ok(())
    }

    pub fn save(&self, history: &ClipboardHistory) -> Result<(), StorageError> {
        debug!(path = %self.path.display(), len = history.len(), "Saving history to disk");

        let bytes = serde_json::to_vec_pretty(history)?;

        // Write to temp file first, then rename for atomicity
        let temp_path = self.path.with_extension("json.tmp");
        std::fs::write(&temp_path, &bytes)?;
        std::fs::rename(&temp_path, &self.path)?;

        info!(bytes = bytes.len(), "History saved");
        Ok(())
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

        let bytes = serde_json::to_vec_pretty(config).map_err(StorageError::Json)?;
        let temp_path = self.config_path.with_extension("json.tmp");
        std::fs::write(&temp_path, &bytes)?;
        std::fs::rename(&temp_path, &self.config_path)?;

        info!("Config saved");
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
    use crate::schema::{MimeType, SensitiveInfo};
    use tempfile::tempdir;

    #[test]
    fn save_and_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("history.json");

        let mut history = ClipboardHistory::new(10, 10, 1024 * 1024);
        history.push(
            MimeType::TextPlain,
            b"test data".to_vec(),
            SensitiveInfo::normal(),
        );

        let storage = Storage {
            path: path.clone(),
            config_path: dir.path().join("config.json"),
        };

        storage.save(&history).unwrap();
        let loaded = storage.load().unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.iter().next().unwrap().as_text(), Some("test data"));
    }

    #[test]
    fn migrate_binary_to_json() {
        let dir = tempdir().unwrap();
        let bin_path = dir.path().join("history.bin");
        let json_path = dir.path().join("history.json");

        // Write history in old binary format
        let mut history = ClipboardHistory::new(10, 10, 1024 * 1024);
        history.push(
            MimeType::TextPlain,
            b"migrated data".to_vec(),
            SensitiveInfo::normal(),
        );
        let bytes = OxiCodeCodec::serialize(&history).unwrap();
        std::fs::write(&bin_path, &bytes).unwrap();

        // Storage points at history.json — load should migrate
        let storage = Storage {
            path: json_path.clone(),
            config_path: dir.path().join("config.json"),
        };

        let loaded = storage.load().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(
            loaded.iter().next().unwrap().as_text(),
            Some("migrated data")
        );

        // history.json should now exist
        assert!(json_path.exists(), "history.json should be created");
        // history.bin should be renamed to backup
        assert!(
            !bin_path.exists(),
            "history.bin should be removed after migration"
        );
        assert!(
            dir.path().join("history.bin.bak").exists(),
            "history.bin.bak should exist"
        );

        // Verify the JSON file is valid JSON
        let json_bytes = std::fs::read(&json_path).unwrap();
        serde_json::from_slice::<ClipboardHistory>(&json_bytes)
            .expect("history.json should be valid JSON");
    }
}
