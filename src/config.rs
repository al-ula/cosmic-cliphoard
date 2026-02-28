// SPDX-License-Identifier: MPL-2.0

//! Keybinding configuration for the clipboard overlay.

use cosmic::iced_core::keyboard::{self, Modifiers, key::Named};

pub use crate::schema::{KeyBinding, KeybindingsConfig};

pub type Config = KeybindingsConfig;

pub trait KeyBindingExt {
    fn matches(&self, key: &keyboard::Key, modifiers: &Modifiers) -> bool;
}

impl KeyBindingExt for KeyBinding {
    fn matches(&self, key: &keyboard::Key, modifiers: &Modifiers) -> bool {
        if modifiers.control() != self.ctrl
            || modifiers.alt() != self.alt
            || modifiers.shift() != self.shift
        {
            return false;
        }

        match key {
            keyboard::Key::Named(named) => self.key == named_key_to_str(named),
            keyboard::Key::Character(c) => c.as_ref().eq_ignore_ascii_case(&self.key),
            _ => false,
        }
    }
}

fn named_key_to_str(named: &Named) -> &'static str {
    match named {
        Named::Delete => "Delete",
        Named::ArrowLeft => "ArrowLeft",
        Named::ArrowRight => "ArrowRight",
        Named::ArrowUp => "ArrowUp",
        Named::ArrowDown => "ArrowDown",
        Named::Enter => "Enter",
        Named::Escape => "Escape",
        Named::Backspace => "Backspace",
        Named::Tab => "Tab",
        Named::Space => "Space",
        Named::Home => "Home",
        Named::End => "End",
        Named::PageUp => "PageUp",
        Named::PageDown => "PageDown",
        Named::Insert => "Insert",
        Named::F1 => "F1",
        Named::F2 => "F2",
        Named::F3 => "F3",
        Named::F4 => "F4",
        Named::F5 => "F5",
        Named::F6 => "F6",
        Named::F7 => "F7",
        Named::F8 => "F8",
        Named::F9 => "F9",
        Named::F10 => "F10",
        Named::F11 => "F11",
        Named::F12 => "F12",
        _ => "",
    }
}

pub fn load_config() -> KeybindingsConfig {
    let config_path = dirs::config_dir().map(|d| d.join("cliphoard").join("keybindings.json"));

    let Some(path) = config_path else {
        return KeybindingsConfig::default();
    };

    if !path.exists() {
        return KeybindingsConfig::default();
    }

    match std::fs::read(&path) {
        Ok(bytes) if !bytes.is_empty() => {
            match serde_json::from_slice::<KeybindingsConfig>(&bytes) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("Failed to parse keybindings config: {e}");
                    KeybindingsConfig::default()
                }
            }
        }
        _ => KeybindingsConfig::default(),
    }
}

pub fn is_first_launch() -> bool {
    let Some(config_dir) = dirs::config_dir() else {
        return true;
    };
    !config_dir.join("cliphoard").join("first_launch_done").exists()
}

pub fn mark_first_launch_done() {
    if let Some(config_dir) = dirs::config_dir() {
        let dir = config_dir.join("cliphoard");
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join("first_launch_done"), b"");
    }
}

pub fn save_keybindings(config: &KeybindingsConfig) -> Result<(), String> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| "Failed to get config directory".to_string())?
        .join("cliphoard");

    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config directory: {e}"))?;

    let path = config_dir.join("keybindings.json");
    let bytes = serde_json::to_vec_pretty(config)
        .map_err(|e| format!("Failed to serialize keybindings: {e}"))?;

    let temp_path = path.with_extension("json.tmp");
    std::fs::write(&temp_path, &bytes).map_err(|e| format!("Failed to write keybindings: {e}"))?;
    std::fs::rename(&temp_path, &path).map_err(|e| format!("Failed to save keybindings: {e}"))?;

    Ok(())
}
