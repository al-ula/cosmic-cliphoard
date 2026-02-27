// SPDX-License-Identifier: MPL-2.0

//! Clipboard history collection with split limits for pinned and unpinned entries.

use crate::entry::{ClipboardEntry, EntryId};
use crate::{DEFAULT_MAX_ENTRY_SIZE, DEFAULT_MAX_PINNED, DEFAULT_MAX_UNPINNED};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Result of attempting to pin an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinResult {
    /// Entry was successfully pinned.
    Pinned,
    /// Entry with the given ID was not found.
    NotFound,
    /// The maximum number of pinned entries has been reached.
    LimitReached,
}

#[derive(Debug, Clone, Serialize, Deserialize, oxicode::Encode, oxicode::Decode)]
pub struct ClipboardHistory {
    entries: VecDeque<ClipboardEntry>,
    /// Maximum number of unpinned entries. Accepts legacy `max_entries` field on deserialization.
    #[serde(alias = "max_entries")]
    max_unpinned: usize,
    #[serde(default = "default_max_pinned")]
    max_pinned: usize,
    #[serde(default = "default_max_entry_size")]
    max_entry_size: usize,
    next_id: u64,
}

fn default_max_pinned() -> usize {
    DEFAULT_MAX_PINNED
}

fn default_max_entry_size() -> usize {
    DEFAULT_MAX_ENTRY_SIZE
}

impl Default for ClipboardHistory {
    fn default() -> Self {
        Self::new(
            DEFAULT_MAX_UNPINNED,
            DEFAULT_MAX_PINNED,
            DEFAULT_MAX_ENTRY_SIZE,
        )
    }
}

impl ClipboardHistory {
    pub fn new(max_unpinned: usize, max_pinned: usize, max_entry_size: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max_unpinned.min(1024)),
            max_unpinned,
            max_pinned,
            max_entry_size,
            next_id: 1,
        }
    }

    /// Push a new entry. Returns `None` if the data exceeds `max_entry_size`.
    pub fn push(&mut self, mime: crate::MimeType, data: Vec<u8>) -> Option<EntryId> {
        if data.len() > self.max_entry_size {
            return None;
        }

        let id = self.next_id;
        self.next_id += 1;

        let entry = ClipboardEntry::new(id, mime, data);
        self.entries.push_front(entry);
        self.evict();
        Some(EntryId(id))
    }

    fn evict(&mut self) {
        while self.unpinned_count() > self.max_unpinned {
            if let Some(pos) = self.entries.iter().rposition(|e| !e.pinned) {
                self.entries.remove(pos);
            } else {
                break;
            }
        }
    }

    pub fn get(&self, id: EntryId) -> Option<&ClipboardEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    pub fn get_mut(&mut self, id: EntryId) -> Option<&mut ClipboardEntry> {
        self.entries.iter_mut().find(|e| e.id == id)
    }

    pub fn remove(&mut self, id: EntryId) -> bool {
        if let Some(pos) = self.entries.iter().position(|e| e.id == id) {
            self.entries.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn pin(&mut self, id: EntryId) -> PinResult {
        if let Some(entry) = self.entries.iter().find(|e| e.id == id) {
            if entry.pinned {
                return PinResult::Pinned;
            }
            if self.pinned_count() >= self.max_pinned {
                return PinResult::LimitReached;
            }
            // Re-find as mutable (borrow rules)
            let entry = self.entries.iter_mut().find(|e| e.id == id).unwrap();
            entry.pinned = true;
            PinResult::Pinned
        } else {
            PinResult::NotFound
        }
    }

    pub fn unpin(&mut self, id: EntryId) -> bool {
        if let Some(entry) = self.get_mut(id) {
            entry.pinned = false;
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        self.entries.retain(|e| e.pinned);
    }

    pub fn iter(&self) -> impl Iterator<Item = &ClipboardEntry> {
        self.entries.iter()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn pinned_count(&self) -> usize {
        self.entries.iter().filter(|e| e.pinned).count()
    }

    pub fn unpinned_count(&self) -> usize {
        self.entries.iter().filter(|e| !e.pinned).count()
    }

    /// Update limits at runtime (e.g. after config reload). Triggers eviction if needed.
    pub fn update_limits(&mut self, max_unpinned: usize, max_pinned: usize, max_entry_size: usize) {
        self.max_unpinned = max_unpinned;
        self.max_pinned = max_pinned;
        self.max_entry_size = max_entry_size;
        self.evict();
    }

    pub fn search(&self, query: &str) -> Vec<&ClipboardEntry> {
        let query_lower = query.to_lowercase();
        self.entries
            .iter()
            .filter(|e| {
                e.as_text()
                    .is_some_and(|t| t.to_lowercase().contains(&query_lower))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MimeType;

    const BIG: usize = 1024 * 1024; // 1 MiB max entry size for tests

    fn test_history(max_unpinned: usize, max_pinned: usize) -> ClipboardHistory {
        ClipboardHistory::new(max_unpinned, max_pinned, BIG)
    }

    #[test]
    fn push_and_evict() {
        let mut history = test_history(3, 10);
        history.push(MimeType::TextPlain, b"first".to_vec());
        history.push(MimeType::TextPlain, b"second".to_vec());
        history.push(MimeType::TextPlain, b"third".to_vec());
        assert_eq!(history.len(), 3);

        // Fourth entry should evict the oldest unpinned.
        history.push(MimeType::TextPlain, b"fourth".to_vec());
        assert_eq!(history.len(), 3);

        // "first" should be gone.
        assert!(history.get(EntryId(1)).is_none());
        // "fourth" should be present.
        assert!(history.get(EntryId(4)).is_some());
    }

    #[test]
    fn pinned_entries_survive_eviction() {
        let mut history = test_history(2, 10);
        let id1 = history
            .push(MimeType::TextPlain, b"pinned".to_vec())
            .unwrap();
        history.pin(id1);
        history.push(MimeType::TextPlain, b"second".to_vec());
        history.push(MimeType::TextPlain, b"third".to_vec());

        // Pinned entry survives, oldest unpinned is evicted.
        // 1 pinned + 2 unpinned, max_unpinned=2, so len=3.
        assert!(history.get(id1).is_some());
        assert_eq!(history.unpinned_count(), 2);
    }

    #[test]
    fn clear_keeps_pinned() {
        let mut history = test_history(10, 10);
        let id1 = history.push(MimeType::TextPlain, b"keep".to_vec()).unwrap();
        history.pin(id1);
        history.push(MimeType::TextPlain, b"gone".to_vec());
        history.clear();
        assert_eq!(history.len(), 1);
        assert!(history.get(id1).is_some());
    }

    #[test]
    fn search_case_insensitive() {
        let mut history = test_history(10, 10);
        history.push(MimeType::TextPlain, b"Hello World".to_vec());
        history.push(MimeType::TextPlain, b"goodbye".to_vec());
        let results = history.search("hello");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_text(), Some("Hello World"));
    }

    #[test]
    fn pin_limit_reached() {
        let mut history = test_history(10, 2);
        let id1 = history.push(MimeType::TextPlain, b"a".to_vec()).unwrap();
        let id2 = history.push(MimeType::TextPlain, b"b".to_vec()).unwrap();
        let id3 = history.push(MimeType::TextPlain, b"c".to_vec()).unwrap();

        assert_eq!(history.pin(id1), PinResult::Pinned);
        assert_eq!(history.pin(id2), PinResult::Pinned);
        assert_eq!(history.pin(id3), PinResult::LimitReached);
        assert_eq!(history.pinned_count(), 2);
    }

    #[test]
    fn pin_already_pinned_succeeds() {
        let mut history = test_history(10, 2);
        let id1 = history.push(MimeType::TextPlain, b"a".to_vec()).unwrap();
        assert_eq!(history.pin(id1), PinResult::Pinned);
        // Pinning again should succeed without counting double.
        assert_eq!(history.pin(id1), PinResult::Pinned);
        assert_eq!(history.pinned_count(), 1);
    }

    #[test]
    fn pin_not_found() {
        let mut history = test_history(10, 10);
        assert_eq!(history.pin(EntryId(999)), PinResult::NotFound);
    }

    #[test]
    fn oversized_entry_rejected() {
        let mut history = ClipboardHistory::new(10, 10, 16);
        // 16 bytes should fit
        assert!(history.push(MimeType::TextPlain, vec![0; 16]).is_some());
        // 17 bytes should be rejected
        assert!(history.push(MimeType::TextPlain, vec![0; 17]).is_none());
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn update_limits_triggers_eviction() {
        let mut history = test_history(10, 10);
        for i in 0..8 {
            history.push(MimeType::TextPlain, format!("entry{i}").into_bytes());
        }
        assert_eq!(history.unpinned_count(), 8);

        history.update_limits(3, 10, BIG);
        assert_eq!(history.unpinned_count(), 3);
    }

    #[test]
    fn serde_migration_from_max_entries() {
        // Simulate old format with `max_entries` field.
        let json = r#"{
            "entries": [],
            "max_entries": 200,
            "next_id": 1
        }"#;
        let history: ClipboardHistory = serde_json::from_str(json).unwrap();
        assert_eq!(history.max_unpinned, 200);
        assert_eq!(history.max_pinned, DEFAULT_MAX_PINNED);
        assert_eq!(history.max_entry_size, DEFAULT_MAX_ENTRY_SIZE);
    }
}
