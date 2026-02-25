// SPDX-License-Identifier: MPL-2.0

//! Clipboard history collection with size limits.

use crate::entry::{ClipboardEntry, EntryId};
use crate::DEFAULT_MAX_ENTRIES;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Ordered clipboard history with a configurable maximum entry count.
///
/// Used directly as a Rust type in lib mode. For IPC, serialize via the
/// [`Codec`](crate::codec::Codec) trait using either bincode or JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(bincode::Encode, bincode::Decode)]
pub struct ClipboardHistory {
    entries: VecDeque<ClipboardEntry>,
    max_entries: usize,
    next_id: u64,
}

impl Default for ClipboardHistory {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_ENTRIES)
    }
}

impl ClipboardHistory {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max_entries.min(1024)),
            max_entries,
            next_id: 1,
        }
    }

    /// Push a new entry, evicting the oldest non-pinned entry if at capacity.
    /// Returns the assigned [`EntryId`].
    pub fn push(&mut self, mime: crate::MimeType, data: Vec<u8>) -> EntryId {
        let id = self.next_id;
        self.next_id += 1;

        let entry = ClipboardEntry::new(id, mime, data);
        self.entries.push_front(entry);
        self.evict();
        EntryId(id)
    }

    /// Remove the oldest non-pinned entry if over capacity.
    fn evict(&mut self) {
        while self.entries.len() > self.max_entries {
            // Find the last non-pinned entry to remove.
            if let Some(pos) = self.entries.iter().rposition(|e| !e.pinned) {
                self.entries.remove(pos);
            } else {
                // All entries are pinned — allow exceeding max.
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

    pub fn pin(&mut self, id: EntryId) -> bool {
        if let Some(entry) = self.get_mut(id) {
            entry.pinned = true;
            true
        } else {
            false
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

    /// Iterate entries newest-first.
    pub fn iter(&self) -> impl Iterator<Item = &ClipboardEntry> {
        self.entries.iter()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Search entries by text content (case-insensitive substring match).
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

    #[test]
    fn push_and_evict() {
        let mut history = ClipboardHistory::new(3);
        history.push(MimeType::TextPlain, b"first".to_vec());
        history.push(MimeType::TextPlain, b"second".to_vec());
        history.push(MimeType::TextPlain, b"third".to_vec());
        assert_eq!(history.len(), 3);

        // Fourth entry should evict the oldest.
        history.push(MimeType::TextPlain, b"fourth".to_vec());
        assert_eq!(history.len(), 3);

        // "first" should be gone.
        assert!(history.get(EntryId(1)).is_none());
        // "fourth" should be present.
        assert!(history.get(EntryId(4)).is_some());
    }

    #[test]
    fn pinned_entries_survive_eviction() {
        let mut history = ClipboardHistory::new(2);
        let id1 = history.push(MimeType::TextPlain, b"pinned".to_vec());
        history.pin(id1);
        history.push(MimeType::TextPlain, b"second".to_vec());
        history.push(MimeType::TextPlain, b"third".to_vec());

        // Pinned entry survives, unpinned oldest is evicted.
        assert!(history.get(id1).is_some());
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn clear_keeps_pinned() {
        let mut history = ClipboardHistory::new(10);
        let id1 = history.push(MimeType::TextPlain, b"keep".to_vec());
        history.pin(id1);
        history.push(MimeType::TextPlain, b"gone".to_vec());
        history.clear();
        assert_eq!(history.len(), 1);
        assert!(history.get(id1).is_some());
    }

    #[test]
    fn search_case_insensitive() {
        let mut history = ClipboardHistory::new(10);
        history.push(MimeType::TextPlain, b"Hello World".to_vec());
        history.push(MimeType::TextPlain, b"goodbye".to_vec());
        let results = history.search("hello");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_text(), Some("Hello World"));
    }
}
