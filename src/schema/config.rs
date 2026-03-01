// SPDX-License-Identifier: MPL-2.0

//! Configuration types for the clipboard manager.

use super::{DEFAULT_MAX_ENTRY_SIZE, DEFAULT_MAX_PINNED, DEFAULT_MAX_UNPINNED};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardConfig {
    #[serde(default = "default_max_unpinned")]
    pub max_unpinned: usize,
    #[serde(default = "default_max_pinned")]
    pub max_pinned: usize,
    #[serde(default = "default_max_entry_size")]
    pub max_entry_size: usize,
}

fn default_max_unpinned() -> usize {
    DEFAULT_MAX_UNPINNED
}

fn default_max_pinned() -> usize {
    DEFAULT_MAX_PINNED
}

fn default_max_entry_size() -> usize {
    DEFAULT_MAX_ENTRY_SIZE
}

impl Default for ClipboardConfig {
    fn default() -> Self {
        Self {
            max_unpinned: DEFAULT_MAX_UNPINNED,
            max_pinned: DEFAULT_MAX_PINNED,
            max_entry_size: DEFAULT_MAX_ENTRY_SIZE,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct KeyBinding {
    pub key: String,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl Serialize for KeyBinding {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("ctrl".to_string());
        }
        if self.alt {
            parts.push("alt".to_string());
        }
        if self.shift {
            parts.push("shift".to_string());
        }
        parts.push(self.key.clone());
        parts.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for KeyBinding {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let parts: Vec<String> = Vec::deserialize(deserializer)?;
        if parts.is_empty() {
            return Err(serde::de::Error::custom("keybinding array cannot be empty"));
        }

        let mut ctrl = false;
        let mut alt = false;
        let mut shift = false;

        for part in &parts[..parts.len() - 1] {
            match part.to_lowercase().as_str() {
                "ctrl" | "control" => ctrl = true,
                "alt" => alt = true,
                "shift" => shift = true,
                other => {
                    return Err(serde::de::Error::custom(format!(
                        "unknown modifier: {other}"
                    )));
                }
            }
        }

        let key = parts.last().unwrap().clone();

        Ok(KeyBinding {
            key,
            ctrl,
            alt,
            shift,
        })
    }
}

impl fmt::Display for KeyBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("ctrl");
        }
        if self.alt {
            parts.push("alt");
        }
        if self.shift {
            parts.push("shift");
        }
        parts.push(&self.key);
        write!(f, "{}", parts.join("+"))
    }
}

impl FromStr for KeyBinding {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('+').collect();
        if parts.is_empty() {
            return Err("keybinding cannot be empty".to_string());
        }

        let mut ctrl = false;
        let mut alt = false;
        let mut shift = false;
        let mut key: Option<String> = None;

        for part in parts {
            match part.to_lowercase().as_str() {
                "ctrl" | "control" => ctrl = true,
                "alt" => alt = true,
                "shift" => shift = true,
                _ => {
                    if let Some(existing) = &key {
                        return Err(format!("multiple keys specified: {existing} and {part}"));
                    }
                    key = Some(part.to_string());
                }
            }
        }

        let key = key.ok_or("no key specified")?;

        Ok(KeyBinding {
            key,
            ctrl,
            alt,
            shift,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct KeybindingsConfig {
    #[serde(default = "default_toggle_pin")]
    pub toggle_pin: KeyBinding,
    #[serde(default = "default_toggle_preview")]
    pub toggle_preview: KeyBinding,
    #[serde(default = "default_delete_entry")]
    pub delete_entry: KeyBinding,
    #[serde(default = "default_tab_all")]
    pub tab_all: KeyBinding,
    #[serde(default = "default_tab_pinned")]
    pub tab_pinned: KeyBinding,
}

fn default_toggle_pin() -> KeyBinding {
    KeyBinding {
        key: "p".into(),
        ctrl: true,
        alt: false,
        shift: false,
    }
}

fn default_toggle_preview() -> KeyBinding {
    KeyBinding {
        key: "e".into(),
        ctrl: true,
        alt: false,
        shift: false,
    }
}

fn default_delete_entry() -> KeyBinding {
    KeyBinding {
        key: "Delete".into(),
        ctrl: false,
        alt: false,
        shift: false,
    }
}

fn default_tab_all() -> KeyBinding {
    KeyBinding {
        key: "ArrowLeft".into(),
        ctrl: false,
        alt: false,
        shift: true,
    }
}

fn default_tab_pinned() -> KeyBinding {
    KeyBinding {
        key: "ArrowRight".into(),
        ctrl: false,
        alt: false,
        shift: true,
    }
}

impl Default for KeybindingsConfig {
    fn default() -> Self {
        Self {
            toggle_pin: default_toggle_pin(),
            toggle_preview: default_toggle_preview(),
            delete_entry: default_delete_entry(),
            tab_all: default_tab_all(),
            tab_pinned: default_tab_pinned(),
        }
    }
}
