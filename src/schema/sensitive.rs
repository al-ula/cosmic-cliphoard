// SPDX-License-Identifier: MPL-2.0

use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    oxicode::Encode,
    oxicode::Decode,
    Default,
)]
pub enum SensitiveState {
    #[default]
    Normal,
    PasswordMime,
    PasswordHeuristic,
    Token,
    Secret,
}

impl SensitiveState {
    #[allow(dead_code)]
    pub fn is_sensitive(&self) -> bool {
        !matches!(self, Self::Normal)
    }
}

#[derive(
    Debug, Clone, PartialEq, Default, Serialize, Deserialize, oxicode::Encode, oxicode::Decode,
)]
pub struct DetectionMethod {
    pub mime_hint: Option<String>,
    pub entropy_score: Option<f64>,
    pub token_pattern: Option<String>,
}

#[derive(
    Debug, Clone, PartialEq, Default, Serialize, Deserialize, oxicode::Encode, oxicode::Decode,
)]
pub struct SensitiveInfo {
    pub state: SensitiveState,
    pub method: DetectionMethod,
}

impl SensitiveInfo {
    pub fn normal() -> Self {
        Self::default()
    }

    pub fn from_mime_hint(mime: &str) -> Self {
        Self {
            state: SensitiveState::PasswordMime,
            method: DetectionMethod {
                mime_hint: Some(mime.to_owned()),
                entropy_score: None,
                token_pattern: None,
            },
        }
    }

    pub fn from_heuristic(entropy: f64) -> Self {
        Self {
            state: SensitiveState::PasswordHeuristic,
            method: DetectionMethod {
                mime_hint: None,
                entropy_score: Some(entropy),
                token_pattern: None,
            },
        }
    }

    pub fn from_token(pattern: &str) -> Self {
        Self {
            state: SensitiveState::Token,
            method: DetectionMethod {
                mime_hint: None,
                entropy_score: None,
                token_pattern: Some(pattern.to_owned()),
            },
        }
    }

    #[allow(dead_code)]
    pub fn secret() -> Self {
        Self {
            state: SensitiveState::Secret,
            method: DetectionMethod::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectionConfig {
    pub mime_hints: bool,
    pub heuristics: bool,
    pub tokens: bool,
}

impl Default for DetectionConfig {
    fn default() -> Self {
        Self {
            mime_hints: true,
            heuristics: false,
            tokens: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitive_state_default_is_normal() {
        assert_eq!(SensitiveState::default(), SensitiveState::Normal);
    }

    #[test]
    fn normal_state_is_not_sensitive() {
        assert!(!SensitiveState::Normal.is_sensitive());
    }

    #[test]
    fn other_states_are_sensitive() {
        assert!(SensitiveState::PasswordMime.is_sensitive());
        assert!(SensitiveState::PasswordHeuristic.is_sensitive());
        assert!(SensitiveState::Token.is_sensitive());
        assert!(SensitiveState::Secret.is_sensitive());
    }

    #[test]
    fn sensitive_info_normal() {
        let info = SensitiveInfo::normal();
        assert_eq!(info.state, SensitiveState::Normal);
        assert!(info.method.mime_hint.is_none());
        assert!(info.method.entropy_score.is_none());
        assert!(info.method.token_pattern.is_none());
    }

    #[test]
    fn sensitive_info_from_mime_hint() {
        let info = SensitiveInfo::from_mime_hint("x-kde-password");
        assert_eq!(info.state, SensitiveState::PasswordMime);
        assert_eq!(info.method.mime_hint, Some("x-kde-password".to_owned()));
    }

    #[test]
    fn sensitive_info_from_heuristic() {
        let info = SensitiveInfo::from_heuristic(4.8);
        assert_eq!(info.state, SensitiveState::PasswordHeuristic);
        assert_eq!(info.method.entropy_score, Some(4.8));
    }

    #[test]
    fn sensitive_info_from_token() {
        let info = SensitiveInfo::from_token("jwt");
        assert_eq!(info.state, SensitiveState::Token);
        assert_eq!(info.method.token_pattern, Some("jwt".to_owned()));
    }

    #[test]
    fn detection_config_defaults() {
        let config = DetectionConfig::default();
        assert!(config.mime_hints);
        assert!(!config.heuristics);
        assert!(config.tokens);
    }
}
