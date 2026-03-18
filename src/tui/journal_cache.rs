use crate::storage::JournalEntry;

#[derive(Debug, Clone, Default)]
pub struct JournalCache {
    pub entries: Vec<JournalEntry>,
}

impl JournalCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_entries(&mut self, entries: Vec<JournalEntry>) {
        self.entries = entries;
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn get_entry(&self, index: usize) -> Option<&JournalEntry> {
        self.entries.get(index)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
