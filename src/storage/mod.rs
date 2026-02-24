pub mod md;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::Local;

pub struct JournalEntry {
    pub filename: String,
    pub path: PathBuf,
}

pub struct DirectoryEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

pub trait JournalStorage {
    fn journal_dir(&self) -> PathBuf;
    fn today_journal_path(&self) -> PathBuf;
    fn open_or_create_today_journal(&self) -> Result<(PathBuf, String)>;
    fn list_journal_entries(&self) -> Result<Vec<JournalEntry>>;
    fn read_journal_entry(&self, path: &Path) -> Result<String>;
    fn save_journal_entry(&self, path: &Path, content: &str) -> Result<()>;
}

pub trait WorkspaceStorage {
    fn programs_dir(&self) -> PathBuf;
    fn list_programs(&self) -> Result<Vec<DirectoryEntry>>;
    fn create_program(&self, name: &str) -> Result<PathBuf>;
    fn list_projects(&self, program: &str) -> Result<Vec<DirectoryEntry>>;
    fn create_project(&self, program: &str, name: &str) -> Result<PathBuf>;
    fn list_milestones(&self, program: &str, project: &str) -> Result<Vec<DirectoryEntry>>;
    fn create_milestone(&self, program: &str, project: &str, name: &str) -> Result<PathBuf>;
    fn list_tasks(
        &self,
        program: &str,
        project: &str,
        milestone: &str,
    ) -> Result<Vec<DirectoryEntry>>;
    fn create_task(
        &self,
        program: &str,
        project: &str,
        milestone: &str,
        name: &str,
    ) -> Result<PathBuf>;
    fn get_task_path(&self, program: &str, project: &str, milestone: &str, task: &str) -> PathBuf;
}

impl JournalStorage for PathBuf {
    fn journal_dir(&self) -> PathBuf {
        self.join("journal")
    }

    fn today_journal_path(&self) -> PathBuf {
        let today = Local::now().format("%Y-%m-%d");
        self.journal_dir().join(format!("{}.md", today))
    }

    fn open_or_create_today_journal(&self) -> Result<(PathBuf, String)> {
        let path = self.today_journal_path();

        if path.exists() {
            let content = fs::read_to_string(&path)?;
            Ok((path, content))
        } else {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let content = String::new();
            fs::write(&path, &content)?;
            Ok((path, content))
        }
    }

    fn list_journal_entries(&self) -> Result<Vec<JournalEntry>> {
        let journal_dir = self.journal_dir();

        if !journal_dir.exists() {
            return Ok(vec![]);
        }

        let mut entries: Vec<JournalEntry> = fs::read_dir(&journal_dir)?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension()?.to_str()? == "md" {
                    let filename = path.file_name()?.to_str()?.to_string();
                    Some(JournalEntry { filename, path })
                } else {
                    None
                }
            })
            .collect();

        entries.sort_by(|a, b| b.filename.cmp(&a.filename));

        Ok(entries)
    }

    fn read_journal_entry(&self, path: &Path) -> Result<String> {
        Ok(fs::read_to_string(path)?)
    }

    fn save_journal_entry(&self, path: &Path, content: &str) -> Result<()> {
        fs::write(path, content)?;
        Ok(())
    }
}

impl WorkspaceStorage for PathBuf {
    fn programs_dir(&self) -> PathBuf {
        self.join("programs")
    }

    fn list_programs(&self) -> Result<Vec<DirectoryEntry>> {
        let programs_dir = self.programs_dir();

        if !programs_dir.exists() {
            return Ok(vec![]);
        }

        let mut entries: Vec<DirectoryEntry> = fs::read_dir(&programs_dir)?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name()?.to_str()?.to_string();
                    Some(DirectoryEntry {
                        name,
                        path,
                        is_dir: true,
                    })
                } else {
                    None
                }
            })
            .collect();

        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    fn create_program(&self, name: &str) -> Result<PathBuf> {
        let program_dir = self.programs_dir().join(name);
        fs::create_dir_all(&program_dir)?;
        Ok(program_dir)
    }

    fn list_projects(&self, program: &str) -> Result<Vec<DirectoryEntry>> {
        let projects_dir = self.programs_dir().join(program);

        if !projects_dir.exists() {
            return Ok(vec![]);
        }

        let mut entries: Vec<DirectoryEntry> = fs::read_dir(&projects_dir)?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name()?.to_str()?.to_string();
                    Some(DirectoryEntry {
                        name,
                        path,
                        is_dir: true,
                    })
                } else {
                    None
                }
            })
            .collect();

        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    fn create_project(&self, program: &str, name: &str) -> Result<PathBuf> {
        let project_dir = self.programs_dir().join(program).join(name);
        fs::create_dir_all(&project_dir)?;
        Ok(project_dir)
    }

    fn list_milestones(&self, program: &str, project: &str) -> Result<Vec<DirectoryEntry>> {
        let milestones_dir = self.programs_dir().join(program).join(project);

        if !milestones_dir.exists() {
            return Ok(vec![]);
        }

        let mut entries: Vec<DirectoryEntry> = fs::read_dir(&milestones_dir)?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name()?.to_str()?.to_string();
                    Some(DirectoryEntry {
                        name,
                        path,
                        is_dir: true,
                    })
                } else {
                    None
                }
            })
            .collect();

        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    fn create_milestone(&self, program: &str, project: &str, name: &str) -> Result<PathBuf> {
        let milestone_dir = self.programs_dir().join(program).join(project).join(name);
        fs::create_dir_all(&milestone_dir)?;
        Ok(milestone_dir)
    }

    fn list_tasks(
        &self,
        program: &str,
        project: &str,
        milestone: &str,
    ) -> Result<Vec<DirectoryEntry>> {
        let tasks_dir = self
            .programs_dir()
            .join(program)
            .join(project)
            .join(milestone);

        if !tasks_dir.exists() {
            return Ok(vec![]);
        }

        let mut entries: Vec<DirectoryEntry> = fs::read_dir(&tasks_dir)?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                let name = path.file_name()?.to_str()?.to_string();
                let is_md = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e == "md")
                    .unwrap_or(false);

                if is_md {
                    Some(DirectoryEntry {
                        name: name.trim_end_matches(".md").to_string(),
                        path,
                        is_dir: false,
                    })
                } else {
                    None
                }
            })
            .collect();

        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    fn create_task(
        &self,
        program: &str,
        project: &str,
        milestone: &str,
        name: &str,
    ) -> Result<PathBuf> {
        let task_path = self.get_task_path(program, project, milestone, name);
        if let Some(parent) = task_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&task_path, "")?;
        Ok(task_path)
    }

    fn get_task_path(&self, program: &str, project: &str, milestone: &str, task: &str) -> PathBuf {
        self.programs_dir()
            .join(program)
            .join(project)
            .join(milestone)
            .join(format!("{}.md", task))
    }
}
