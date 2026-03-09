pub mod md;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::iter;
use std::path::{Component, Path, PathBuf};

use crate::error::{Result, StorageError};
use chrono::Local;

pub struct JournalEntry {
    pub filename: String,
    pub path: PathBuf,
}

#[derive(Clone)]
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

// TODO: WorkspaceStorage trait defines a complete API for element manipulation.
// Many methods are not yet used by the TUI but are implemented for future features.
// When wiring up element modification commands, these methods will be used.
pub trait WorkspaceStorage {
    fn programs_dir(&self) -> PathBuf;
    fn list_programs(&self) -> Result<Vec<DirectoryEntry>>;
    fn read_program(&self, name: &str) -> Result<String>;
    fn save_program(&self, name: &str, content: &str) -> Result<()>;
    fn create_program(&self, name: &str, description: &str) -> Result<PathBuf>;
    fn list_projects(&self, program: &str) -> Result<Vec<DirectoryEntry>>;
    fn read_project(&self, program: &str, name: &str) -> Result<String>;
    fn save_project(&self, program: &str, name: &str, content: &str) -> Result<()>;
    fn create_project(&self, program: &str, name: &str, description: &str) -> Result<PathBuf>;
    fn list_milestones(&self, program: &str, project: &str) -> Result<Vec<DirectoryEntry>>;
    fn read_milestone(&self, program: &str, project: &str, name: &str) -> Result<String>;
    fn save_milestone(&self, program: &str, project: &str, name: &str, content: &str)
    -> Result<()>;
    fn create_milestone(
        &self,
        program: &str,
        project: &str,
        name: &str,
        description: &str,
    ) -> Result<PathBuf>;
    fn list_tasks(
        &self,
        program: &str,
        project: &str,
        milestone: &str,
    ) -> Result<Vec<DirectoryEntry>>;
    fn list_subtasks(
        &self,
        program: &str,
        project: &str,
        milestone: &str,
        task: &str,
    ) -> Result<Vec<DirectoryEntry>>;
    fn read_task(
        &self,
        program: &str,
        project: &str,
        milestone: &str,
        name: &str,
    ) -> Result<String>;
    fn save_task(
        &self,
        program: &str,
        project: &str,
        milestone: &str,
        name: &str,
        content: &str,
    ) -> Result<()>;
    fn create_task(
        &self,
        program: &str,
        project: &str,
        milestone: &str,
        name: &str,
        description: &str,
    ) -> Result<PathBuf>;
    fn get_task_path(&self, program: &str, project: &str, milestone: &str, task: &str) -> PathBuf;
    fn read_md_file(&self, path: &Path) -> Result<String>;
    fn create_from_template(
        &self,
        template_name: &str,
        target_path: &Path,
        values: &HashMap<String, String>,
        strip_labels: &HashSet<String>,
    ) -> Result<PathBuf>;
}

pub fn validate_element_name(name: &str) -> Result<()> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(StorageError::InvalidPath("Element name cannot be empty".to_string()).into());
    }

    if trimmed
        .chars()
        .any(|c| c == '/' || c == '\\' || c.is_control())
    {
        return Err(StorageError::InvalidPath(format!(
            "Element name '{}' contains invalid path characters",
            name
        ))
        .into());
    }

    if trimmed.contains("..") {
        return Err(StorageError::InvalidPath(format!(
            "Element name '{}' cannot contain '..'",
            name
        ))
        .into());
    }

    let mut components = Path::new(trimmed).components();
    match components.next() {
        Some(Component::Normal(_)) if components.next().is_none() => Ok(()),
        _ => Err(StorageError::InvalidPath(format!(
            "Element name '{}' is not a valid filename",
            name
        ))
        .into()),
    }
}

fn validate_target_path(workspace_root: &Path, target_path: &Path) -> Result<()> {
    let absolute_target = if target_path.is_absolute() {
        target_path.to_path_buf()
    } else {
        workspace_root.join(target_path)
    };

    let relative = absolute_target.strip_prefix(workspace_root).map_err(|_| {
        StorageError::InvalidPath(format!(
            "Target path '{}' is outside workspace '{}'",
            absolute_target.display(),
            workspace_root.display()
        ))
    })?;

    if relative.components().any(|c| {
        matches!(
            c,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(StorageError::InvalidPath(format!(
            "Target path '{}' escapes workspace '{}'",
            absolute_target.display(),
            workspace_root.display()
        ))
        .into());
    }

    Ok(())
}

fn create_named_element(
    workspace: &PathBuf,
    template_name: &str,
    target_path: &Path,
    name: &str,
    description: &str,
) -> Result<PathBuf> {
    let mut values = HashMap::new();
    values.insert("NAME".to_string(), name.to_string());
    values.insert("DESCRIPTION".to_string(), description.to_string());
    values.insert(
        "TODAY".to_string(),
        Local::now().format("%Y-%m-%d").to_string(),
    );
    values.insert("DEFAULT_STATUS".to_string(), "todo".to_string());
    values.insert("OWNER".to_string(), String::new());
    values.insert("ASSIGNED_TO".to_string(), String::new());
    values.insert("DUE_DATE".to_string(), String::new());
    values.insert("PROGRAM_NAME".to_string(), name.to_string());
    values.insert("PROJECT_NAME".to_string(), name.to_string());
    values.insert("MILESTONE_NAME".to_string(), name.to_string());
    values.insert("TASK_NAME".to_string(), name.to_string());

    let strip_labels: HashSet<String> = HashSet::new();
    workspace.create_from_template(template_name, target_path, &values, &strip_labels)
}

fn program_markdown_path(workspace: &Path, program: &str) -> PathBuf {
    workspace
        .join("programs")
        .join(program)
        .join(format!("{}.md", program))
}

fn legacy_program_markdown_path(workspace: &Path, program: &str) -> PathBuf {
    workspace.join("programs").join(format!("{}.md", program))
}

fn project_markdown_path(workspace: &Path, program: &str, project: &str) -> PathBuf {
    workspace
        .join("programs")
        .join(program)
        .join("projects")
        .join(project)
        .join(format!("{}.md", project))
}

fn legacy_project_markdown_path(workspace: &Path, program: &str, project: &str) -> PathBuf {
    workspace
        .join("programs")
        .join(program)
        .join(format!("{}.md", project))
}

fn milestone_markdown_path(
    workspace: &Path,
    program: &str,
    project: &str,
    milestone: &str,
) -> PathBuf {
    workspace
        .join("programs")
        .join(program)
        .join("projects")
        .join(project)
        .join("milestones")
        .join(milestone)
        .join(format!("{}.md", milestone))
}

fn legacy_milestone_markdown_path(
    workspace: &Path,
    program: &str,
    project: &str,
    milestone: &str,
) -> PathBuf {
    workspace
        .join("programs")
        .join(program)
        .join(project)
        .join(format!("{}.md", milestone))
}

fn task_markdown_path(
    workspace: &Path,
    program: &str,
    project: &str,
    milestone: &str,
    task: &str,
) -> PathBuf {
    workspace
        .join("programs")
        .join(program)
        .join("projects")
        .join(project)
        .join("milestones")
        .join(milestone)
        .join("tasks")
        .join(task)
        .join(format!("{}.md", task))
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
            let template = include_str!("../../templates/journal.md");
            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
            let content = template.replace("YYYY-MM-DD", &today);
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
                let name = path.file_name()?.to_str()?.to_string();

                // Structure 1: Flat .md file (programs/MyProgram.md)
                if name.ends_with(".md") {
                    return Some(DirectoryEntry {
                        name: name.trim_end_matches(".md").to_string(),
                        path,
                        is_dir: false,
                    });
                }

                // Structure 2: Nested (programs/MyProgram/MyProgram.md)
                if path.is_dir() {
                    let nested_md = path.join(format!("{}.md", name));
                    if nested_md.exists() {
                        return Some(DirectoryEntry {
                            name,
                            path: nested_md,
                            is_dir: true,
                        });
                    }
                }

                None
            })
            .collect();

        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    fn read_program(&self, name: &str) -> Result<String> {
        let canonical = program_markdown_path(self, name);
        let path = if canonical.exists() {
            canonical
        } else {
            legacy_program_markdown_path(self, name)
        };
        Ok(fs::read_to_string(path)?)
    }

    fn save_program(&self, name: &str, content: &str) -> Result<()> {
        validate_element_name(name)?;
        let path = program_markdown_path(self, name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }

    fn create_program(&self, name: &str, description: &str) -> Result<PathBuf> {
        validate_element_name(name)?;
        let program_md = program_markdown_path(self, name);
        let program_dir = self.programs_dir().join(name);
        fs::create_dir_all(&program_dir)?;
        create_named_element(self, "program", &program_md, name, description)?;
        Ok(program_md)
    }

    fn list_projects(&self, program: &str) -> Result<Vec<DirectoryEntry>> {
        let program_dir = self.programs_dir().join(program);
        discover_across_bases(
            iter::once(program_dir),
            &format!("{}.md", program),
            "projects",
        )
    }

    fn read_project(&self, program: &str, name: &str) -> Result<String> {
        let canonical = project_markdown_path(self, program, name);
        let path = if canonical.exists() {
            canonical
        } else if let Ok(entries) = self.list_projects(program) {
            entries
                .into_iter()
                .find(|entry| entry.name == name)
                .map(|entry| entry.path)
                .unwrap_or_else(|| legacy_project_markdown_path(self, program, name))
        } else {
            legacy_project_markdown_path(self, program, name)
        };
        Ok(fs::read_to_string(path)?)
    }

    fn save_project(&self, program: &str, name: &str, content: &str) -> Result<()> {
        validate_element_name(program)?;
        validate_element_name(name)?;
        let path = project_markdown_path(self, program, name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }

    fn create_project(&self, program: &str, name: &str, description: &str) -> Result<PathBuf> {
        validate_element_name(program)?;
        validate_element_name(name)?;
        let project_md = project_markdown_path(self, program, name);
        let project_dir = self
            .programs_dir()
            .join(program)
            .join("projects")
            .join(name);
        fs::create_dir_all(&project_dir)?;
        create_named_element(self, "project", &project_md, name, description)?;
        Ok(project_md)
    }

    fn list_milestones(&self, program: &str, project: &str) -> Result<Vec<DirectoryEntry>> {
        discover_across_bases(
            [
                self.programs_dir().join(program).join(project),
                self.programs_dir()
                    .join(program)
                    .join("projects")
                    .join(project),
            ],
            &format!("{}.md", project),
            "milestones",
        )
    }

    fn read_milestone(&self, program: &str, project: &str, name: &str) -> Result<String> {
        let canonical = milestone_markdown_path(self, program, project, name);
        let path = if canonical.exists() {
            canonical
        } else if let Ok(entries) = self.list_milestones(program, project) {
            entries
                .into_iter()
                .find(|entry| entry.name == name)
                .map(|entry| entry.path)
                .unwrap_or_else(|| legacy_milestone_markdown_path(self, program, project, name))
        } else {
            legacy_milestone_markdown_path(self, program, project, name)
        };
        Ok(fs::read_to_string(path)?)
    }

    fn save_milestone(
        &self,
        program: &str,
        project: &str,
        name: &str,
        content: &str,
    ) -> Result<()> {
        validate_element_name(program)?;
        validate_element_name(project)?;
        validate_element_name(name)?;
        let path = milestone_markdown_path(self, program, project, name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }

    fn create_milestone(
        &self,
        program: &str,
        project: &str,
        name: &str,
        description: &str,
    ) -> Result<PathBuf> {
        validate_element_name(program)?;
        validate_element_name(project)?;
        validate_element_name(name)?;
        let milestone_md = milestone_markdown_path(self, program, project, name);
        let milestone_dir = self
            .programs_dir()
            .join(program)
            .join("projects")
            .join(project)
            .join("milestones")
            .join(name);
        fs::create_dir_all(&milestone_dir)?;
        create_named_element(self, "milestone", &milestone_md, name, description)?;
        Ok(milestone_md)
    }

    fn list_tasks(
        &self,
        program: &str,
        project: &str,
        milestone: &str,
    ) -> Result<Vec<DirectoryEntry>> {
        discover_across_bases(
            [
                self.programs_dir()
                    .join(program)
                    .join(project)
                    .join(milestone),
                self.programs_dir()
                    .join(program)
                    .join("projects")
                    .join(project)
                    .join("milestones")
                    .join(milestone),
            ],
            &format!("{}.md", milestone),
            "tasks",
        )
    }

    fn list_subtasks(
        &self,
        program: &str,
        project: &str,
        milestone: &str,
        task: &str,
    ) -> Result<Vec<DirectoryEntry>> {
        discover_across_bases(
            [
                self.programs_dir()
                    .join(program)
                    .join(project)
                    .join(milestone)
                    .join("tasks")
                    .join(task),
                self.programs_dir()
                    .join(program)
                    .join("projects")
                    .join(project)
                    .join("milestones")
                    .join(milestone)
                    .join("tasks")
                    .join(task),
            ],
            &format!("{}.md", task),
            "subtasks",
        )
    }

    fn read_task(
        &self,
        program: &str,
        project: &str,
        milestone: &str,
        name: &str,
    ) -> Result<String> {
        let canonical = self.get_task_path(program, project, milestone, name);
        let path = if canonical.exists() {
            canonical
        } else if let Ok(entries) = self.list_tasks(program, project, milestone) {
            entries
                .into_iter()
                .find(|entry| entry.name == name)
                .map(|entry| entry.path)
                .unwrap_or_else(|| canonical)
        } else {
            canonical
        };
        Ok(fs::read_to_string(path)?)
    }

    fn save_task(
        &self,
        program: &str,
        project: &str,
        milestone: &str,
        name: &str,
        content: &str,
    ) -> Result<()> {
        validate_element_name(program)?;
        validate_element_name(project)?;
        validate_element_name(milestone)?;
        validate_element_name(name)?;
        let path = self.get_task_path(program, project, milestone, name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }

    fn create_task(
        &self,
        program: &str,
        project: &str,
        milestone: &str,
        name: &str,
        description: &str,
    ) -> Result<PathBuf> {
        validate_element_name(program)?;
        validate_element_name(project)?;
        validate_element_name(milestone)?;
        validate_element_name(name)?;
        let task_path = self.get_task_path(program, project, milestone, name);
        if let Some(parent) = task_path.parent() {
            fs::create_dir_all(parent)?;
        }
        create_named_element(self, "task", &task_path, name, description)?;
        Ok(task_path)
    }

    fn get_task_path(&self, program: &str, project: &str, milestone: &str, task: &str) -> PathBuf {
        task_markdown_path(self, program, project, milestone, task)
    }

    fn read_md_file(&self, path: &Path) -> Result<String> {
        Ok(fs::read_to_string(path)?)
    }

    fn create_from_template(
        &self,
        template_name: &str,
        target_path: &Path,
        values: &HashMap<String, String>,
        strip_labels: &HashSet<String>,
    ) -> Result<PathBuf> {
        tracing::debug!(
            template = template_name,
            target = %target_path.display(),
            value_keys = values.len(),
            strip_labels = strip_labels.len(),
            "creating element from template"
        );
        validate_target_path(self, target_path)?;

        let template = match template_name {
            "program" => include_str!("../../templates/program.md"),
            "project" => include_str!("../../templates/project.md"),
            "milestone" => include_str!("../../templates/milestone.md"),
            "task" => include_str!("../../templates/task.md"),
            "subtask" => include_str!("../../templates/subtask.md"),
            "journal" => include_str!("../../templates/journal.md"),
            _ => return Err(StorageError::TemplateNotFound(template_name.to_string()).into()),
        };

        let content = resolve_template(template, values, strip_labels);

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target_path, content)?;
        tracing::debug!(
            template = template_name,
            target = %target_path.display(),
            "template write complete"
        );
        Ok(target_path.to_path_buf())
    }
}

pub fn parse_template_fields(template: &str) -> Vec<(String, String, bool)> {
    let mut fields = Vec::new();
    let mut seen_placeholders: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Only handle inline YAML format: "field: {{Placeholder}}"
    // This is the only supported format

    let re_inline = regex::Regex::new(r"\{\{(\w+)\}\}").unwrap();

    // Track if we're past the YAML frontmatter (after the closing ---)
    let mut in_yaml = false;
    let mut found_yaml_end = false;

    for line in template.lines() {
        let line_trimmed = line.trim();

        // Track YAML boundaries
        if line_trimmed == "---" {
            if !in_yaml {
                in_yaml = true;
            } else {
                found_yaml_end = true;
                in_yaml = false;
            }
            continue;
        }

        // Skip comments and empty lines
        if line_trimmed.starts_with('#') || line_trimmed.is_empty() {
            continue;
        }

        // Check for inline placeholders: "field: {{Placeholder}}"
        // Only extract if it looks like a YAML field (has colon before the placeholder)
        if let Some(colon_pos) = line_trimmed.find(':') {
            let before_colon = &line_trimmed[..colon_pos];
            let after_colon = &line_trimmed[colon_pos + 1..];

            // Check if there's a placeholder after the colon
            if let Some(cap) = re_inline.captures(after_colon)
                && let Some(placeholder_match) = cap.get(1)
            {
                let placeholder = placeholder_match.as_str().to_string();
                if !placeholder.is_empty() && !seen_placeholders.contains(&placeholder) {
                    // Extract label from text before the colon
                    let label = extract_label_from_yaml_line(before_colon);
                    seen_placeholders.insert(placeholder.clone());
                    fields.push((label, placeholder, true));
                }
            }
        }

        // Also detect {{DESCRIPTION}} outside YAML frontmatter (in markdown body)
        if found_yaml_end
            && !in_yaml
            && let Some(cap) = re_inline.captures(line_trimmed)
            && let Some(placeholder_match) = cap.get(1)
        {
            let placeholder = placeholder_match.as_str().to_string();
            if placeholder == "DESCRIPTION" && !seen_placeholders.contains(&placeholder) {
                seen_placeholders.insert(placeholder.clone());
                // DESCRIPTION in markdown body should be stripped from YAML and put in body
                fields.push(("Description".to_string(), placeholder, true));
            }
        }
    }

    tracing::debug!(
        field_count = fields.len(),
        placeholders = ?fields
            .iter()
            .map(|(_, placeholder, strip)| (placeholder.clone(), *strip))
            .collect::<Vec<_>>(),
        "parsed template fields"
    );
    fields
}

/// Extract label from YAML field name (e.g., "creation_date" -> "Creation Date")
fn extract_label_from_yaml_line(field_name: &str) -> String {
    // Convert field name to title case
    // e.g., "creation_date" -> "Creation Date", "created_by" -> "Created By"
    field_name
        .replace('_', " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn resolve_template(
    template: &str,
    values: &HashMap<String, String>,
    strip_labels: &HashSet<String>,
) -> String {
    let today = Local::now().format("%Y-%m-%d").to_string();

    let mut result = template.to_string();

    let re = regex::Regex::new(r"\{\{(\w+)\}\}").unwrap();

    result = re
        .replace_all(&result, |caps: &regex::Captures| {
            let placeholder = &caps[1];

            if placeholder == "TODAY" {
                return today.clone();
            }

            if placeholder == "UUID" {
                return uuid::Uuid::new_v4().to_string();
            }

            values.get(placeholder).cloned().unwrap_or_default()
        })
        .to_string();

    if !strip_labels.is_empty() {
        let re_strip = regex::Regex::new(r"#([^:!]+)![:]?\s*(.*)").unwrap();
        result = re_strip.replace_all(&result, "$2").to_string();
    }

    result
}

/// Discover elements in a directory, handling both flat and nested structures.
///
/// # Arguments
/// * `base_dir` - The directory to search in
/// * `parent_md_name` - The parent's .md filename to exclude (e.g., "Milestone.md")
/// * `subdir_name` - The subdirectory name for nested elements (e.g., "tasks" or "subtasks")
fn discover_elements(
    base_dir: &Path,
    parent_md_name: &str,
    subdir_name: &str,
) -> Result<Vec<DirectoryEntry>> {
    let mut entries: Vec<DirectoryEntry> = Vec::new();

    if !base_dir.exists() {
        return Ok(entries);
    }

    // Check flat structure in base directory
    let flat_entries: Vec<DirectoryEntry> = fs::read_dir(base_dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let name = path.file_name()?.to_str()?.to_string();

            // Flat .md file
            if name.ends_with(".md") && name != parent_md_name {
                return Some(DirectoryEntry {
                    name: name.trim_end_matches(".md").to_string(),
                    path,
                    is_dir: false,
                });
            }

            // Nested directory with .md inside
            if path.is_dir() && name != subdir_name {
                let nested_md = path.join(format!("{}.md", name));
                if nested_md.exists() {
                    return Some(DirectoryEntry {
                        name,
                        path: nested_md,
                        is_dir: true,
                    });
                }
            }

            None
        })
        .collect();
    entries.extend(flat_entries);

    // Check subdirectory for more elements
    let subdir = base_dir.join(subdir_name);
    if subdir.exists() {
        let nested_entries: Vec<DirectoryEntry> = fs::read_dir(&subdir)?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                let name = path.file_name()?.to_str()?.to_string();

                // Flat .md file in subdirectory
                if name.ends_with(".md") {
                    return Some(DirectoryEntry {
                        name: name.trim_end_matches(".md").to_string(),
                        path,
                        is_dir: false,
                    });
                }

                // Nested directory in subdirectory
                if path.is_dir() {
                    let nested_md = path.join(format!("{}.md", name));
                    if nested_md.exists() {
                        return Some(DirectoryEntry {
                            name,
                            path: nested_md,
                            is_dir: true,
                        });
                    }
                }

                None
            })
            .collect();
        entries.extend(nested_entries);
    }

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    tracing::debug!(
        base = %base_dir.display(),
        parent_md = parent_md_name,
        subdir = subdir_name,
        discovered = entries.len(),
        "discovered element entries"
    );
    Ok(entries)
}

fn discover_across_bases<I>(
    base_dirs: I,
    parent_md_name: &str,
    subdir_name: &str,
) -> Result<Vec<DirectoryEntry>>
where
    I: IntoIterator<Item = PathBuf>,
{
    let mut all = Vec::new();
    for base_dir in base_dirs {
        let mut entries = discover_elements(&base_dir, parent_md_name, subdir_name)?;
        all.append(&mut entries);
    }

    let mut seen = HashSet::new();
    all.retain(|entry| seen.insert(entry.name.clone()));
    all.sort_by(|a, b| a.name.cmp(&b.name));
    tracing::debug!(
        parent_md = parent_md_name,
        subdir = subdir_name,
        deduped = all.len(),
        "discovered entries across base dirs"
    );
    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_flat_program(temp_dir: &Path, name: &str) {
        let programs_dir = temp_dir.join("programs");
        fs::create_dir_all(&programs_dir).unwrap();
        fs::write(programs_dir.join(format!("{}.md", name)), "# Flat Program").unwrap();
    }

    fn create_nested_program(temp_dir: &Path, name: &str) {
        let program_dir = temp_dir.join("programs").join(name);
        fs::create_dir_all(&program_dir).unwrap();
        fs::write(program_dir.join(format!("{}.md", name)), "# Nested Program").unwrap();
    }

    fn create_flat_project(temp_dir: &Path, program: &str, name: &str) {
        let program_dir = temp_dir.join("programs").join(program);
        fs::create_dir_all(&program_dir).unwrap();
        fs::write(program_dir.join(format!("{}.md", name)), "# Flat Project").unwrap();
    }

    fn create_nested_project(temp_dir: &Path, program: &str, name: &str) {
        let project_dir = temp_dir
            .join("programs")
            .join(program)
            .join("projects")
            .join(name);
        fs::create_dir_all(&project_dir).unwrap();
        fs::write(project_dir.join(format!("{}.md", name)), "# Nested Project").unwrap();
    }

    fn create_flat_milestone(temp_dir: &Path, program: &str, project: &str, name: &str) {
        let project_dir = temp_dir.join("programs").join(program).join(project);
        fs::create_dir_all(&project_dir).unwrap();
        fs::write(project_dir.join(format!("{}.md", name)), "# Flat Milestone").unwrap();
    }

    fn create_nested_milestone(temp_dir: &Path, program: &str, project: &str, name: &str) {
        let milestone_dir = temp_dir
            .join("programs")
            .join(program)
            .join("projects")
            .join(project)
            .join("milestones")
            .join(name);
        fs::create_dir_all(&milestone_dir).unwrap();
        fs::write(
            milestone_dir.join(format!("{}.md", name)),
            "# Nested Milestone",
        )
        .unwrap();
    }

    fn create_flat_task(
        temp_dir: &Path,
        program: &str,
        project: &str,
        milestone: &str,
        name: &str,
    ) {
        let milestone_dir = temp_dir
            .join("programs")
            .join(program)
            .join(project)
            .join(milestone);
        fs::create_dir_all(&milestone_dir).unwrap();
        fs::write(milestone_dir.join(format!("{}.md", name)), "# Flat Task").unwrap();
    }

    fn create_nested_task(
        temp_dir: &Path,
        program: &str,
        project: &str,
        milestone: &str,
        name: &str,
    ) {
        let task_dir = temp_dir
            .join("programs")
            .join(program)
            .join("projects")
            .join(project)
            .join("milestones")
            .join(milestone)
            .join("tasks")
            .join(name);
        fs::create_dir_all(&task_dir).unwrap();
        fs::write(task_dir.join(format!("{}.md", name)), "# Nested Task").unwrap();
    }

    fn create_flat_task_in_tasks_subdir(
        temp_dir: &Path,
        program: &str,
        project: &str,
        milestone: &str,
        name: &str,
    ) {
        let tasks_dir = temp_dir
            .join("programs")
            .join(program)
            .join("projects")
            .join(project)
            .join("milestones")
            .join(milestone)
            .join("tasks");
        fs::create_dir_all(&tasks_dir).unwrap();
        fs::write(
            tasks_dir.join(format!("{}.md", name)),
            "# Flat Task in Tasks Dir",
        )
        .unwrap();
    }

    fn create_nested_subtask(
        temp_dir: &Path,
        program: &str,
        project: &str,
        milestone: &str,
        task: &str,
        name: &str,
    ) {
        let subtask_dir = temp_dir
            .join("programs")
            .join(program)
            .join("projects")
            .join(project)
            .join("milestones")
            .join(milestone)
            .join("tasks")
            .join(task)
            .join("subtasks")
            .join(name);
        fs::create_dir_all(&subtask_dir).unwrap();
        fs::write(subtask_dir.join(format!("{}.md", name)), "# Nested Subtask").unwrap();
    }

    fn create_flat_subtask_in_subtasks_subdir(
        temp_dir: &Path,
        program: &str,
        project: &str,
        milestone: &str,
        task: &str,
        name: &str,
    ) {
        let subtasks_dir = temp_dir
            .join("programs")
            .join(program)
            .join("projects")
            .join(project)
            .join("milestones")
            .join(milestone)
            .join("tasks")
            .join(task)
            .join("subtasks");
        fs::create_dir_all(&subtasks_dir).unwrap();
        fs::write(
            subtasks_dir.join(format!("{}.md", name)),
            "# Flat Subtask in Subtasks Dir",
        )
        .unwrap();
    }

    #[test]
    fn test_list_programs_flat() {
        let temp_dir = TempDir::new().unwrap();
        create_flat_program(temp_dir.path(), "FlatProgram");

        let programs = temp_dir.path().to_path_buf().list_programs().unwrap();
        assert_eq!(programs.len(), 1);
        assert_eq!(programs[0].name, "FlatProgram");
        assert!(!programs[0].is_dir);
    }

    #[test]
    fn test_list_programs_nested() {
        let temp_dir = TempDir::new().unwrap();
        create_nested_program(temp_dir.path(), "NestedProgram");

        let programs = temp_dir.path().to_path_buf().list_programs().unwrap();
        assert_eq!(programs.len(), 1);
        assert_eq!(programs[0].name, "NestedProgram");
        assert!(programs[0].is_dir);
    }

    #[test]
    fn test_list_programs_mixed() {
        let temp_dir = TempDir::new().unwrap();
        create_flat_program(temp_dir.path(), "FlatProgram");
        create_nested_program(temp_dir.path(), "NestedProgram");

        let programs = temp_dir.path().to_path_buf().list_programs().unwrap();
        assert_eq!(programs.len(), 2);
        let names: Vec<&str> = programs.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"FlatProgram"));
        assert!(names.contains(&"NestedProgram"));
    }

    #[test]
    fn test_list_projects_flat() {
        let temp_dir = TempDir::new().unwrap();
        create_nested_program(temp_dir.path(), "MyProgram");
        create_flat_project(temp_dir.path(), "MyProgram", "FlatProject");

        let projects = temp_dir
            .path()
            .to_path_buf()
            .list_projects("MyProgram")
            .unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "FlatProject");
    }

    #[test]
    fn test_list_projects_nested_in_subdir() {
        let temp_dir = TempDir::new().unwrap();
        create_nested_program(temp_dir.path(), "MyProgram");
        create_nested_project(temp_dir.path(), "MyProgram", "NestedProject");

        let projects = temp_dir
            .path()
            .to_path_buf()
            .list_projects("MyProgram")
            .unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "NestedProject");
    }

    #[test]
    fn test_list_projects_mixed() {
        let temp_dir = TempDir::new().unwrap();
        create_nested_program(temp_dir.path(), "MyProgram");
        create_flat_project(temp_dir.path(), "MyProgram", "FlatProject");
        create_nested_project(temp_dir.path(), "MyProgram", "NestedProject");

        let projects = temp_dir
            .path()
            .to_path_buf()
            .list_projects("MyProgram")
            .unwrap();
        assert_eq!(projects.len(), 2);
        let names: Vec<&str> = projects.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"FlatProject"));
        assert!(names.contains(&"NestedProject"));
    }

    #[test]
    fn test_list_milestones_flat() {
        let temp_dir = TempDir::new().unwrap();
        create_nested_program(temp_dir.path(), "MyProgram");
        create_nested_project(temp_dir.path(), "MyProgram", "MyProject");
        create_flat_milestone(temp_dir.path(), "MyProgram", "MyProject", "FlatMilestone");

        let milestones = temp_dir
            .path()
            .to_path_buf()
            .list_milestones("MyProgram", "MyProject")
            .unwrap();
        assert_eq!(milestones.len(), 1);
        assert_eq!(milestones[0].name, "FlatMilestone");
    }

    #[test]
    fn test_list_milestones_nested_in_subdir() {
        let temp_dir = TempDir::new().unwrap();
        create_nested_program(temp_dir.path(), "MyProgram");
        create_nested_project(temp_dir.path(), "MyProgram", "MyProject");
        create_nested_milestone(temp_dir.path(), "MyProgram", "MyProject", "NestedMilestone");

        let milestones = temp_dir
            .path()
            .to_path_buf()
            .list_milestones("MyProgram", "MyProject")
            .unwrap();
        assert_eq!(milestones.len(), 1);
        assert_eq!(milestones[0].name, "NestedMilestone");
    }

    #[test]
    fn test_list_milestones_mixed() {
        let temp_dir = TempDir::new().unwrap();
        create_nested_program(temp_dir.path(), "MyProgram");
        create_nested_project(temp_dir.path(), "MyProgram", "MyProject");
        create_flat_milestone(temp_dir.path(), "MyProgram", "MyProject", "FlatMilestone");
        create_nested_milestone(temp_dir.path(), "MyProgram", "MyProject", "NestedMilestone");

        let milestones = temp_dir
            .path()
            .to_path_buf()
            .list_milestones("MyProgram", "MyProject")
            .unwrap();
        assert_eq!(milestones.len(), 2);
        let names: Vec<&str> = milestones.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"FlatMilestone"));
        assert!(names.contains(&"NestedMilestone"));
    }

    #[test]
    fn test_list_tasks_flat() {
        let temp_dir = TempDir::new().unwrap();
        create_nested_program(temp_dir.path(), "MyProgram");
        create_nested_project(temp_dir.path(), "MyProgram", "MyProject");
        create_nested_milestone(temp_dir.path(), "MyProgram", "MyProject", "MyMilestone");
        create_flat_task(
            temp_dir.path(),
            "MyProgram",
            "MyProject",
            "MyMilestone",
            "FlatTask",
        );

        let tasks = temp_dir
            .path()
            .to_path_buf()
            .list_tasks("MyProgram", "MyProject", "MyMilestone")
            .unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].name, "FlatTask");
    }

    #[test]
    fn test_list_tasks_nested_in_subdir() {
        let temp_dir = TempDir::new().unwrap();
        create_nested_program(temp_dir.path(), "MyProgram");
        create_nested_project(temp_dir.path(), "MyProgram", "MyProject");
        create_nested_milestone(temp_dir.path(), "MyProgram", "MyProject", "MyMilestone");
        create_nested_task(
            temp_dir.path(),
            "MyProgram",
            "MyProject",
            "MyMilestone",
            "NestedTask",
        );

        let tasks = temp_dir
            .path()
            .to_path_buf()
            .list_tasks("MyProgram", "MyProject", "MyMilestone")
            .unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].name, "NestedTask");
    }

    #[test]
    fn test_list_tasks_mixed() {
        let temp_dir = TempDir::new().unwrap();
        create_nested_program(temp_dir.path(), "MyProgram");
        create_nested_project(temp_dir.path(), "MyProgram", "MyProject");
        create_nested_milestone(temp_dir.path(), "MyProgram", "MyProject", "MyMilestone");
        create_flat_task(
            temp_dir.path(),
            "MyProgram",
            "MyProject",
            "MyMilestone",
            "FlatTask",
        );
        create_nested_task(
            temp_dir.path(),
            "MyProgram",
            "MyProject",
            "MyMilestone",
            "NestedTask",
        );

        let tasks = temp_dir
            .path()
            .to_path_buf()
            .list_tasks("MyProgram", "MyProject", "MyMilestone")
            .unwrap();
        assert_eq!(tasks.len(), 2);
        let names: Vec<&str> = tasks.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"FlatTask"));
        assert!(names.contains(&"NestedTask"));
    }

    #[test]
    fn test_list_programs_empty() {
        let temp_dir = TempDir::new().unwrap();
        let programs = temp_dir.path().to_path_buf().list_programs().unwrap();
        assert!(programs.is_empty());
    }

    #[test]
    fn test_list_projects_empty_program() {
        let temp_dir = TempDir::new().unwrap();
        create_nested_program(temp_dir.path(), "MyProgram");
        let projects = temp_dir
            .path()
            .to_path_buf()
            .list_projects("MyProgram")
            .unwrap();
        assert!(projects.is_empty());
    }

    #[test]
    fn test_list_tasks_flat_in_tasks_subdir() {
        let temp_dir = TempDir::new().unwrap();
        create_nested_program(temp_dir.path(), "MyProgram");
        create_nested_project(temp_dir.path(), "MyProgram", "MyProject");
        create_nested_milestone(temp_dir.path(), "MyProgram", "MyProject", "MyMilestone");
        create_flat_task_in_tasks_subdir(
            temp_dir.path(),
            "MyProgram",
            "MyProject",
            "MyMilestone",
            "FlatTaskInSubdir",
        );

        let tasks = temp_dir
            .path()
            .to_path_buf()
            .list_tasks("MyProgram", "MyProject", "MyMilestone")
            .unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].name, "FlatTaskInSubdir");
        assert!(!tasks[0].is_dir);
    }

    #[test]
    fn test_list_tasks_all_types() {
        let temp_dir = TempDir::new().unwrap();
        create_nested_program(temp_dir.path(), "MyProgram");
        create_nested_project(temp_dir.path(), "MyProgram", "MyProject");
        create_nested_milestone(temp_dir.path(), "MyProgram", "MyProject", "MyMilestone");
        // Flat task directly in milestone
        create_flat_task(
            temp_dir.path(),
            "MyProgram",
            "MyProject",
            "MyMilestone",
            "FlatTask",
        );
        // Flat task in tasks/ subdirectory
        create_flat_task_in_tasks_subdir(
            temp_dir.path(),
            "MyProgram",
            "MyProject",
            "MyMilestone",
            "FlatTaskInSubdir",
        );
        // Nested task in tasks/ subdirectory
        create_nested_task(
            temp_dir.path(),
            "MyProgram",
            "MyProject",
            "MyMilestone",
            "NestedTask",
        );

        let tasks = temp_dir
            .path()
            .to_path_buf()
            .list_tasks("MyProgram", "MyProject", "MyMilestone")
            .unwrap();
        assert_eq!(tasks.len(), 3);
        let names: Vec<&str> = tasks.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"FlatTask"));
        assert!(names.contains(&"FlatTaskInSubdir"));
        assert!(names.contains(&"NestedTask"));
    }

    #[test]
    fn test_list_subtasks_nested() {
        let temp_dir = TempDir::new().unwrap();
        create_nested_program(temp_dir.path(), "MyProgram");
        create_nested_project(temp_dir.path(), "MyProgram", "MyProject");
        create_nested_milestone(temp_dir.path(), "MyProgram", "MyProject", "MyMilestone");
        create_nested_task(
            temp_dir.path(),
            "MyProgram",
            "MyProject",
            "MyMilestone",
            "MyTask",
        );
        create_nested_subtask(
            temp_dir.path(),
            "MyProgram",
            "MyProject",
            "MyMilestone",
            "MyTask",
            "NestedSubtask",
        );

        let subtasks = temp_dir
            .path()
            .to_path_buf()
            .list_subtasks("MyProgram", "MyProject", "MyMilestone", "MyTask")
            .unwrap();
        assert_eq!(subtasks.len(), 1);
        assert_eq!(subtasks[0].name, "NestedSubtask");
        assert!(subtasks[0].is_dir);
    }

    #[test]
    fn test_list_subtasks_flat_in_subtasks_subdir() {
        let temp_dir = TempDir::new().unwrap();
        create_nested_program(temp_dir.path(), "MyProgram");
        create_nested_project(temp_dir.path(), "MyProgram", "MyProject");
        create_nested_milestone(temp_dir.path(), "MyProgram", "MyProject", "MyMilestone");
        create_nested_task(
            temp_dir.path(),
            "MyProgram",
            "MyProject",
            "MyMilestone",
            "MyTask",
        );
        create_flat_subtask_in_subtasks_subdir(
            temp_dir.path(),
            "MyProgram",
            "MyProject",
            "MyMilestone",
            "MyTask",
            "FlatSubtask",
        );

        let subtasks = temp_dir
            .path()
            .to_path_buf()
            .list_subtasks("MyProgram", "MyProject", "MyMilestone", "MyTask")
            .unwrap();
        assert_eq!(subtasks.len(), 1);
        assert_eq!(subtasks[0].name, "FlatSubtask");
        assert!(!subtasks[0].is_dir);
    }

    #[test]
    fn test_list_subtasks_all_types() {
        let temp_dir = TempDir::new().unwrap();
        create_nested_program(temp_dir.path(), "MyProgram");
        create_nested_project(temp_dir.path(), "MyProgram", "MyProject");
        create_nested_milestone(temp_dir.path(), "MyProgram", "MyProject", "MyMilestone");
        create_nested_task(
            temp_dir.path(),
            "MyProgram",
            "MyProject",
            "MyMilestone",
            "MyTask",
        );
        // Nested subtask
        create_nested_subtask(
            temp_dir.path(),
            "MyProgram",
            "MyProject",
            "MyMilestone",
            "MyTask",
            "NestedSubtask",
        );
        // Flat subtask in subtasks/ subdirectory
        create_flat_subtask_in_subtasks_subdir(
            temp_dir.path(),
            "MyProgram",
            "MyProject",
            "MyMilestone",
            "MyTask",
            "FlatSubtask",
        );

        let subtasks = temp_dir
            .path()
            .to_path_buf()
            .list_subtasks("MyProgram", "MyProject", "MyMilestone", "MyTask")
            .unwrap();
        assert_eq!(subtasks.len(), 2);
        let names: Vec<&str> = subtasks.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"NestedSubtask"));
        assert!(names.contains(&"FlatSubtask"));
    }

    #[test]
    fn test_list_subtasks_empty() {
        let temp_dir = TempDir::new().unwrap();
        create_nested_program(temp_dir.path(), "MyProgram");
        create_nested_project(temp_dir.path(), "MyProgram", "MyProject");
        create_nested_milestone(temp_dir.path(), "MyProgram", "MyProject", "MyMilestone");
        create_nested_task(
            temp_dir.path(),
            "MyProgram",
            "MyProject",
            "MyMilestone",
            "MyTask",
        );

        let subtasks = temp_dir
            .path()
            .to_path_buf()
            .list_subtasks("MyProgram", "MyProject", "MyMilestone", "MyTask")
            .unwrap();
        assert!(subtasks.is_empty());
    }

    #[test]
    fn test_create_project_writes_canonical_path() {
        let temp_dir = TempDir::new().unwrap();
        create_nested_program(temp_dir.path(), "MyProgram");

        let workspace = temp_dir.path().to_path_buf();
        let created = workspace
            .create_project("MyProgram", "MyProject", "Project description")
            .unwrap();

        let canonical = temp_dir
            .path()
            .join("programs")
            .join("MyProgram")
            .join("projects")
            .join("MyProject")
            .join("MyProject.md");
        let legacy = temp_dir
            .path()
            .join("programs")
            .join("MyProgram")
            .join("MyProject.md");

        assert_eq!(created, canonical);
        assert!(canonical.exists());
        assert!(!legacy.exists());
    }

    #[test]
    fn test_save_project_writes_canonical_path() {
        let temp_dir = TempDir::new().unwrap();
        create_nested_program(temp_dir.path(), "MyProgram");

        let workspace = temp_dir.path().to_path_buf();
        workspace
            .save_project("MyProgram", "MyProject", "# Canonical")
            .unwrap();

        let canonical = temp_dir
            .path()
            .join("programs")
            .join("MyProgram")
            .join("projects")
            .join("MyProject")
            .join("MyProject.md");
        assert!(canonical.exists());
        assert_eq!(fs::read_to_string(canonical).unwrap(), "# Canonical");
    }

    #[test]
    fn test_read_task_falls_back_to_legacy_path() {
        let temp_dir = TempDir::new().unwrap();
        create_nested_program(temp_dir.path(), "MyProgram");
        create_nested_project(temp_dir.path(), "MyProgram", "MyProject");
        create_nested_milestone(temp_dir.path(), "MyProgram", "MyProject", "MyMilestone");
        create_flat_task(
            temp_dir.path(),
            "MyProgram",
            "MyProject",
            "MyMilestone",
            "LegacyTask",
        );

        let workspace = temp_dir.path().to_path_buf();
        let content = workspace
            .read_task("MyProgram", "MyProject", "MyMilestone", "LegacyTask")
            .unwrap();

        assert!(content.contains("LegacyTask") || content.contains("Flat Task"));
    }

    #[test]
    fn test_parse_template_fields_detects_description_outside_yaml() {
        // Test template with DESCRIPTION in markdown body (outside YAML)
        let template = r#"---
uuid: {{UUID}}
title: {{NAME}}
status: {{DEFAULT_STATUS}}
---

# Description
{{DESCRIPTION}}
"#;
        let fields = parse_template_fields(template);

        // Should detect both YAML fields and DESCRIPTION
        let placeholders: Vec<&str> = fields.iter().map(|(_, p, _)| p.as_str()).collect();
        assert!(
            placeholders.contains(&"DESCRIPTION"),
            "Should detect DESCRIPTION placeholder, got: {:?}",
            placeholders
        );
    }

    #[test]
    fn test_parse_template_fields_yaml_only() {
        // Test template with only YAML fields (no DESCRIPTION in body)
        let template = r#"---
uuid: {{UUID}}
title: {{NAME}}
status: {{DEFAULT_STATUS}}
tags: program
---

Some markdown content without placeholders.
"#;
        let fields = parse_template_fields(template);

        // Should detect YAML fields but not DESCRIPTION
        let placeholders: Vec<&str> = fields.iter().map(|(_, p, _)| p.as_str()).collect();
        assert!(
            !placeholders.contains(&"DESCRIPTION"),
            "Should NOT detect DESCRIPTION when not present"
        );
        assert!(placeholders.contains(&"UUID"));
        assert!(placeholders.contains(&"NAME"));
    }

    #[test]
    fn test_resolve_template_preserves_description_in_body() {
        let template = r#"---
uuid: {{UUID}}
title: {{NAME}}
status: {{DEFAULT_STATUS}}
---

# Description
{{DESCRIPTION}}
"#;
        let mut values = HashMap::new();
        values.insert("UUID".to_string(), "test-uuid".to_string());
        values.insert("NAME".to_string(), "Test Program".to_string());
        values.insert("DEFAULT_STATUS".to_string(), "New".to_string());
        values.insert(
            "DESCRIPTION".to_string(),
            "This is the description".to_string(),
        );
        let strip_labels: HashSet<String> = HashSet::new();

        let result = resolve_template(template, &values, &strip_labels);

        // Verify DESCRIPTION is replaced in markdown body
        assert!(
            result.contains("This is the description"),
            "Description should appear in markdown body"
        );
        // Verify it's not in YAML
        let yaml_end = result.find("---").unwrap_or(0);
        let after_yaml = &result[yaml_end + 3..];
        assert!(
            !after_yaml.starts_with("description:"),
            "DESCRIPTION should not appear as YAML field"
        );
    }

    #[test]
    fn test_validate_element_name_rejects_path_traversal() {
        assert!(validate_element_name("../escape").is_err());
        assert!(validate_element_name("..").is_err());
        assert!(validate_element_name("safe-name").is_ok());
    }

    #[test]
    fn test_create_from_template_rejects_out_of_workspace_path() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        let mut values = HashMap::new();
        values.insert("NAME".to_string(), "SafeName".to_string());
        let strip_labels: HashSet<String> = HashSet::new();

        let bad_target = workspace.join("programs").join("..").join("evil.md");
        let result = workspace.create_from_template("program", &bad_target, &values, &strip_labels);

        assert!(result.is_err());
    }
}
