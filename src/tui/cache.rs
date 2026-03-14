use crate::model::SelectedTask;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct TaskMetadata {
    pub uuid: String,
    pub path: PathBuf,
    pub program: String,
    pub project: String,
    pub milestone: String,
    pub task_name: String,
    pub status: String,
}

impl From<TaskMetadata> for SelectedTask {
    fn from(meta: TaskMetadata) -> Self {
        SelectedTask {
            uuid: meta.uuid,
            path: meta.path,
            program: meta.program,
            project: meta.project,
            milestone: meta.milestone,
            task_name: meta.task_name,
            status: meta.status,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TaskCache {
    by_uuid: HashMap<String, TaskMetadata>,
    by_path: HashMap<PathBuf, String>,
}

impl TaskCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, metadata: TaskMetadata) {
        let uuid = metadata.uuid.clone();
        let path = metadata.path.clone();
        self.by_uuid.insert(uuid.clone(), metadata);
        self.by_path.insert(path, uuid);
    }

    pub fn get_by_uuid(&self, uuid: &str) -> Option<&TaskMetadata> {
        self.by_uuid.get(uuid)
    }

    pub fn get_by_path(&self, path: &PathBuf) -> Option<&TaskMetadata> {
        self.by_path
            .get(path)
            .and_then(|uuid| self.by_uuid.get(uuid))
    }

    pub fn is_empty(&self) -> bool {
        self.by_uuid.is_empty()
    }

    pub fn len(&self) -> usize {
        self.by_uuid.len()
    }

    pub fn clear(&mut self) {
        self.by_uuid.clear();
        self.by_path.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &TaskMetadata> {
        self.by_uuid.values()
    }
}

pub type SharedTaskCache = Arc<RwLock<TaskCache>>;

pub fn create_shared_cache() -> SharedTaskCache {
    Arc::new(RwLock::new(TaskCache::new()))
}
