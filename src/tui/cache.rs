use crate::model::SelectedTask;
use crate::storage::DirectoryEntry;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct DataCache {
    pub programs: Vec<DirectoryEntry>,
    pub projects: Vec<DirectoryEntry>,
    pub milestones: Vec<DirectoryEntry>,
    pub tasks: Vec<DirectoryEntry>,
    pub subtasks: Vec<DirectoryEntry>,
}

impl DataCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.programs.is_empty()
    }

    pub fn len(&self) -> usize {
        self.programs.len()
    }
}

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
            assigned_to: None,
            start_date: None,
            due_date: None,
            priority: None,
            description: None,
        }
    }
}
