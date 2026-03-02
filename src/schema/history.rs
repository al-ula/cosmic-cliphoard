// SPDX-License-Identifier: MPL-2.0

use crate::schema::MimeType;

use super::entry::{ClipboardEntry, EntryId};
use super::sensitive::SensitiveInfo;
use super::{DEFAULT_MAX_ENTRY_SIZE, DEFAULT_MAX_PINNED, DEFAULT_MAX_UNPINNED};
use serde::{Deserialize, Serialize};
use skim::fuzzy_matcher::FuzzyMatcher;
use skim::fuzzy_matcher::skim::SkimMatcherV2;
use std::collections::VecDeque;

pub fn fuzzy_search<'a>(
    entries: impl Iterator<Item = &'a ClipboardEntry>,
    query: &str,
) -> Vec<&'a ClipboardEntry> {
    if query.is_empty() {
        return entries.collect();
    }
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(i64, &ClipboardEntry)> = entries
        .filter_map(|entry| {
            let text = entry.search_text();
            let score = matcher.fuzzy_match(&text, query)?;
            Some((score, entry))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored.into_iter().map(|(_, e)| e).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinResult {
    Pinned,

    NotFound,

    LimitReached,
}

#[derive(Debug, Clone, Serialize, Deserialize, oxicode::Encode, oxicode::Decode)]
pub struct ClipboardHistory {
    entries: VecDeque<ClipboardEntry>,

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

    pub fn push(
        &mut self,
        mime: MimeType,
        data: Vec<u8>,
        sensitive: SensitiveInfo,
    ) -> Option<EntryId> {
        if data.len() > self.max_entry_size {
            return None;
        }

        let id = self.next_id;
        self.next_id += 1;

        let entry = ClipboardEntry::new(id, mime, data, sensitive);
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

    pub fn pinned_count(&self) -> usize {
        self.entries.iter().filter(|e| e.pinned).count()
    }

    pub fn unpinned_count(&self) -> usize {
        self.entries.iter().filter(|e| !e.pinned).count()
    }

    pub fn update_limits(&mut self, max_unpinned: usize, max_pinned: usize, max_entry_size: usize) {
        self.max_unpinned = max_unpinned;
        self.max_pinned = max_pinned;
        self.max_entry_size = max_entry_size;
        self.evict();
    }

    pub fn search(&self, query: &str) -> Vec<&ClipboardEntry> {
        fuzzy_search(self.entries.iter(), query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use MimeType;

    const BIG: usize = 1024 * 1024;

    fn test_history(max_unpinned: usize, max_pinned: usize) -> ClipboardHistory {
        ClipboardHistory::new(max_unpinned, max_pinned, BIG)
    }

    fn normal() -> SensitiveInfo {
        SensitiveInfo::normal()
    }

    #[test]
    fn push_and_evict() {
        let mut history = test_history(3, 10);
        history.push(MimeType::TextPlain, b"first".to_vec(), normal());
        history.push(MimeType::TextPlain, b"second".to_vec(), normal());
        history.push(MimeType::TextPlain, b"third".to_vec(), normal());
        assert_eq!(history.len(), 3);

        history.push(MimeType::TextPlain, b"fourth".to_vec(), normal());
        assert_eq!(history.len(), 3);

        assert!(history.get(EntryId(1)).is_none());

        assert!(history.get(EntryId(4)).is_some());
    }

    #[test]
    fn pinned_entries_survive_eviction() {
        let mut history = test_history(2, 10);
        let id1 = history
            .push(MimeType::TextPlain, b"pinned".to_vec(), normal())
            .unwrap();
        history.pin(id1);
        history.push(MimeType::TextPlain, b"second".to_vec(), normal());
        history.push(MimeType::TextPlain, b"third".to_vec(), normal());

        assert!(history.get(id1).is_some());
        assert_eq!(history.unpinned_count(), 2);
    }

    #[test]
    fn clear_keeps_pinned() {
        let mut history = test_history(10, 10);
        let id1 = history
            .push(MimeType::TextPlain, b"keep".to_vec(), normal())
            .unwrap();
        history.pin(id1);
        history.push(MimeType::TextPlain, b"gone".to_vec(), normal());
        history.clear();
        assert_eq!(history.len(), 1);
        assert!(history.get(id1).is_some());
    }

    #[test]
    fn search_case_insensitive() {
        let mut history = test_history(10, 10);
        history.push(MimeType::TextPlain, b"Hello World".to_vec(), normal());
        history.push(MimeType::TextPlain, b"goodbye".to_vec(), normal());
        let results = history.search("hello");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_text(), Some("Hello World"));
    }

    #[test]
    fn search_fuzzy_matching() {
        let mut history = test_history(10, 10);
        history.push(MimeType::TextPlain, b"clipboard_manager".to_vec(), normal());
        history.push(MimeType::TextPlain, b"something else".to_vec(), normal());
        let results = history.search("clmgr");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_text(), Some("clipboard_manager"));
    }

    #[test]
    fn search_score_ordering() {
        let mut history = test_history(10, 10);
        history.push(
            MimeType::TextPlain,
            b"xxxxxxxhelloxxxxxxx".to_vec(),
            normal(),
        );
        history.push(MimeType::TextPlain, b"hello".to_vec(), normal());
        let results = history.search("hello");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].as_text(), Some("hello"));
    }

    #[test]
    fn search_empty_query_returns_all() {
        let mut history = test_history(10, 10);
        history.push(MimeType::TextPlain, b"alpha".to_vec(), normal());
        history.push(MimeType::TextPlain, b"beta".to_vec(), normal());
        let results = history.search("");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn search_finds_image_by_mime() {
        let mut history = test_history(10, 10);
        history.push(MimeType::TextPlain, b"some text".to_vec(), normal());
        history.push(MimeType::ImagePng, vec![0x89, 0x50, 0x4E, 0x47], normal());
        let results = history.search("png");
        assert_eq!(results.len(), 1);
        assert!(results[0].mime.is_image());
    }

    #[test]
    fn search_finds_image_by_category() {
        let mut history = test_history(10, 10);
        history.push(MimeType::TextPlain, b"no match".to_vec(), normal());
        history.push(MimeType::ImageJpeg, vec![0xFF, 0xD8, 0xFF], normal());
        let results = history.search("image");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].mime, MimeType::ImageJpeg);
    }

    #[test]
    fn search_empty_returns_all_including_non_text() {
        let mut history = test_history(10, 10);
        history.push(MimeType::TextPlain, b"hello".to_vec(), normal());
        history.push(MimeType::ImagePng, vec![0x89, 0x50, 0x4E, 0x47], normal());
        let results = history.search("");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn pin_limit_reached() {
        let mut history = test_history(10, 2);
        let id1 = history
            .push(MimeType::TextPlain, b"a".to_vec(), normal())
            .unwrap();
        let id2 = history
            .push(MimeType::TextPlain, b"b".to_vec(), normal())
            .unwrap();
        let id3 = history
            .push(MimeType::TextPlain, b"c".to_vec(), normal())
            .unwrap();

        assert_eq!(history.pin(id1), PinResult::Pinned);
        assert_eq!(history.pin(id2), PinResult::Pinned);
        assert_eq!(history.pin(id3), PinResult::LimitReached);
        assert_eq!(history.pinned_count(), 2);
    }

    #[test]
    fn pin_already_pinned_succeeds() {
        let mut history = test_history(10, 2);
        let id1 = history
            .push(MimeType::TextPlain, b"a".to_vec(), normal())
            .unwrap();
        assert_eq!(history.pin(id1), PinResult::Pinned);

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

        assert!(
            history
                .push(MimeType::TextPlain, vec![0; 16], normal())
                .is_some()
        );

        assert!(
            history
                .push(MimeType::TextPlain, vec![0; 17], normal())
                .is_none()
        );
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn update_limits_triggers_eviction() {
        let mut history = test_history(10, 10);
        for i in 0..8 {
            history.push(
                MimeType::TextPlain,
                format!("entry{i}").into_bytes(),
                normal(),
            );
        }
        assert_eq!(history.unpinned_count(), 8);

        history.update_limits(3, 10, BIG);
        assert_eq!(history.unpinned_count(), 3);
    }

    #[test]
    fn serde_migration_from_max_entries() {
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
