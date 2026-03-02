// SPDX-License-Identifier: MPL-2.0

use crate::schema::{DetectionConfig, SensitiveInfo};
use std::collections::HashMap;

const PASSWORD_MIME_TYPES: &[&str] = &[
    "x-kde-password",
    "application/x-password",
    "application/vnd.org.kde.ark.pass",
    "secret/text-plain",
    "password",
    "application/x-kde-passwordmanager",
];

const MIN_PASSWORD_LEN: usize = 8;
const MAX_PASSWORD_LEN: usize = 128;
const MIN_ENTROPY_THRESHOLD: f64 = 3.5;

pub fn detect_sensitive(
    data: &[u8],
    mime_from_wl: &str,
    config: &DetectionConfig,
) -> SensitiveInfo {
    if config.mime_hints
        && let Some(info) = check_mime_hints(mime_from_wl)
    {
        return info;
    }

    if let Ok(text) = std::str::from_utf8(data) {
        if config.tokens
            && let Some(info) = check_token_patterns(text)
        {
            return info;
        }

        if config.heuristics
            && let Some(info) = check_password_heuristics(text)
        {
            return info;
        }
    }

    SensitiveInfo::normal()
}

fn check_mime_hints(mime: &str) -> Option<SensitiveInfo> {
    let mime_lower = mime.to_lowercase();
    for &known in PASSWORD_MIME_TYPES {
        if mime_lower == known || mime_lower.contains(known) {
            return Some(SensitiveInfo::from_mime_hint(mime));
        }
    }
    None
}

fn check_token_patterns(text: &str) -> Option<SensitiveInfo> {
    let trimmed = text.trim();

    if trimmed.len() < 20 || trimmed.contains(char::is_whitespace) {
        return None;
    }

    if is_jwt(trimmed) {
        return Some(SensitiveInfo::from_token("jwt"));
    }

    if is_github_token(trimmed) {
        return Some(SensitiveInfo::from_token("github_token"));
    }

    if is_stripe_key(trimmed) {
        return Some(SensitiveInfo::from_token("stripe_key"));
    }

    if is_aws_key(trimmed) {
        return Some(SensitiveInfo::from_token("aws_access_key"));
    }

    if is_slack_token(trimmed) {
        return Some(SensitiveInfo::from_token("slack_token"));
    }

    if is_gitlab_token(trimmed) {
        return Some(SensitiveInfo::from_token("gitlab_token"));
    }

    if is_npm_token(trimmed) {
        return Some(SensitiveInfo::from_token("npm_token"));
    }

    if is_pypi_token(trimmed) {
        return Some(SensitiveInfo::from_token("pypi_token"));
    }

    if is_discord_token(trimmed) {
        return Some(SensitiveInfo::from_token("discord_token"));
    }

    if is_generic_bearer(trimmed) {
        return Some(SensitiveInfo::from_token("generic_bearer"));
    }

    None
}

fn is_jwt(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    parts[0].starts_with("eyJ")
        && parts[1].starts_with("eyJ")
        && parts[0]
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        && parts[1]
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        && parts[2]
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

fn is_github_token(s: &str) -> bool {
    let prefixes = ["ghp_", "gho_", "ghu_", "ghs_", "ghr_"];
    prefixes.iter().any(|p| s.starts_with(p))
        && s.len() >= 40
        && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

fn is_stripe_key(s: &str) -> bool {
    let prefixes = ["sk_live_", "sk_test_", "rk_live_", "rk_test_"];
    prefixes.iter().any(|p| s.starts_with(p))
        && s.len() >= 32
        && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

fn is_aws_key(s: &str) -> bool {
    s.starts_with("AKIA")
        && s.len() == 20
        && s.chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
}

fn is_slack_token(s: &str) -> bool {
    let prefixes = ["xoxb-", "xoxp-", "xoxa-", "xoxr-"];
    prefixes.iter().any(|p| s.starts_with(p))
        && s.len() >= 50
        && s.chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

fn is_gitlab_token(s: &str) -> bool {
    s.starts_with("glpat-")
        && s.len() >= 26
        && s[6..]
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

fn is_npm_token(s: &str) -> bool {
    s.starts_with("npm_") && s.len() >= 40 && s[4..].chars().all(|c| c.is_alphanumeric())
}

fn is_pypi_token(s: &str) -> bool {
    s.starts_with("pypi-")
        && s.len() >= 55
        && s[5..]
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

fn is_discord_token(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() == 3
        && parts[0].len() >= 24
        && parts[0].chars().all(|c| c.is_alphanumeric())
        && parts[1].len() == 6
        && parts[1].chars().all(|c| c.is_alphanumeric())
        && parts[2].len() >= 27
        && parts[2]
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

fn is_generic_bearer(s: &str) -> bool {
    s.len() >= 32
        && s.chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

fn check_password_heuristics(text: &str) -> Option<SensitiveInfo> {
    let trimmed = text.trim();

    let len = trimmed.chars().count();
    if !(MIN_PASSWORD_LEN..=MAX_PASSWORD_LEN).contains(&len) {
        return None;
    }

    if trimmed.contains(char::is_whitespace) {
        return None;
    }

    let entropy = calculate_entropy(trimmed);
    if entropy < MIN_ENTROPY_THRESHOLD {
        return None;
    }

    let mut has_lower = false;
    let mut has_upper = false;
    let mut has_digit = false;
    let mut has_symbol = false;

    for c in trimmed.chars() {
        match c {
            'a'..='z' => has_lower = true,
            'A'..='Z' => has_upper = true,
            '0'..='9' => has_digit = true,
            _ => has_symbol = true,
        }
    }

    let classes = [has_lower, has_upper, has_digit, has_symbol]
        .iter()
        .filter(|&&x| x)
        .count();

    if classes < 2 {
        return None;
    }

    Some(SensitiveInfo::from_heuristic(entropy))
}

fn calculate_entropy(text: &str) -> f64 {
    if text.is_empty() {
        return 0.0;
    }

    let mut freq: HashMap<char, usize> = HashMap::new();
    let len = text.chars().count();

    for c in text.chars() {
        *freq.entry(c).or_insert(0) += 1;
    }

    let mut entropy = 0.0;
    for &count in freq.values() {
        let p = count as f64 / len as f64;
        entropy -= p * p.log2();
    }

    entropy
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::sensitive::SensitiveState;

    #[test]
    fn detect_password_mime() {
        let config = DetectionConfig::default();
        let result = detect_sensitive(b"secret", "x-kde-password", &config);
        assert_eq!(result.state, SensitiveState::PasswordMime);
    }

    #[test]
    fn detect_jwt_token() {
        let config = DetectionConfig::default();
        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let result = detect_sensitive(jwt.as_bytes(), "text/plain", &config);
        assert_eq!(result.state, SensitiveState::Token);
        assert_eq!(result.method.token_pattern, Some("jwt".to_owned()));
    }

    #[test]
    fn detect_github_token() {
        let config = DetectionConfig::default();
        let token = "ghp_1234567890abcdefghijklmnopqrstuvwxyz";
        let result = detect_sensitive(token.as_bytes(), "text/plain", &config);
        assert_eq!(result.state, SensitiveState::Token);
    }

    #[test]
    fn detect_stripe_key() {
        let config = DetectionConfig::default();
        let key = format!("{}{}", "sk_live_", "x".repeat(28));
        let result = detect_sensitive(key.as_bytes(), "text/plain", &config);
        assert_eq!(result.state, SensitiveState::Token);
    }

    #[test]
    fn detect_aws_key() {
        let config = DetectionConfig::default();
        let key = "AKIAIOSFODNN7EXAMPLE";
        let result = detect_sensitive(key.as_bytes(), "text/plain", &config);
        assert_eq!(result.state, SensitiveState::Token);
    }

    #[test]
    fn no_detect_normal_text() {
        let config = DetectionConfig::default();
        let text = "Hello, this is normal text";
        let result = detect_sensitive(text.as_bytes(), "text/plain", &config);
        assert_eq!(result.state, SensitiveState::Normal);
    }

    #[test]
    fn no_detect_low_entropy() {
        let config = DetectionConfig {
            heuristics: true,
            ..Default::default()
        };
        let text = "aaaaaaaaaaaaaaaa";
        let result = detect_sensitive(text.as_bytes(), "text/plain", &config);
        assert_eq!(result.state, SensitiveState::Normal);
    }

    #[test]
    fn detect_high_entropy_password() {
        let config = DetectionConfig {
            heuristics: true,
            ..Default::default()
        };
        let password = "Kj9#mP2$vL5@nQ8&";
        let result = detect_sensitive(password.as_bytes(), "text/plain", &config);
        assert_eq!(result.state, SensitiveState::PasswordHeuristic);
    }

    #[test]
    fn heuristics_opt_in() {
        let config = DetectionConfig {
            heuristics: false,
            ..Default::default()
        };
        let password = "Kj9#mP2$vL5@nQ8&";
        let result = detect_sensitive(password.as_bytes(), "text/plain", &config);
        assert_eq!(result.state, SensitiveState::Normal);
    }

    #[test]
    fn entropy_calculation() {
        assert!(calculate_entropy("aaaa") < 1.0);
        assert!(calculate_entropy("abcd") > 1.5);
        assert!(calculate_entropy("aA1!") > 1.5);
    }

    #[test]
    fn mime_hints_disabled() {
        let config = DetectionConfig {
            mime_hints: false,
            ..Default::default()
        };
        let result = detect_sensitive(b"secret", "x-kde-password", &config);
        assert_eq!(result.state, SensitiveState::Normal);
    }

    #[test]
    fn tokens_disabled() {
        let config = DetectionConfig {
            tokens: false,
            ..Default::default()
        };
        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let result = detect_sensitive(jwt.as_bytes(), "text/plain", &config);
        assert_eq!(result.state, SensitiveState::Normal);
    }

    #[test]
    fn no_detect_text_with_spaces() {
        let config = DetectionConfig {
            heuristics: true,
            ..Default::default()
        };
        let text = "Kj9# mP2$ vL5@";
        let result = detect_sensitive(text.as_bytes(), "text/plain", &config);
        assert_eq!(result.state, SensitiveState::Normal);
    }

    #[test]
    fn detect_slack_token() {
        let config = DetectionConfig::default();
        let token = format!("{}-{}-{}-{}", "xoxb", "000000000000", "0000000000000", "x".repeat(24));
        let result = detect_sensitive(token.as_bytes(), "text/plain", &config);
        assert_eq!(result.state, SensitiveState::Token);
    }

    #[test]
    fn detect_gitlab_token() {
        let config = DetectionConfig::default();
        let token = "glpat-xxxxxxxxxxxxxxxxxxxx";
        let result = detect_sensitive(token.as_bytes(), "text/plain", &config);
        assert_eq!(result.state, SensitiveState::Token);
    }
}
