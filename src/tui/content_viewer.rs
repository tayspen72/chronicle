use crate::storage::DirectoryEntry;

#[derive(Debug, Clone, Default)]
pub struct ContentViewer {
    pub selected_content: Option<DirectoryEntry>,
    pub current_content_text: Option<String>,
}

impl ContentViewer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self, entry: DirectoryEntry, content: String) {
        self.current_content_text = Some(content);
        self.selected_content = Some(entry);
    }

    pub fn close(&mut self) {
        self.selected_content = None;
        self.current_content_text = None;
    }

    pub fn is_viewing(&self) -> bool {
        self.selected_content.is_some()
    }
}
