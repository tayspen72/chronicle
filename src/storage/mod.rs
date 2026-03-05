pub mod md;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

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
    #[allow(dead_code)]
    pub is_dir: bool,
}

pub trait JournalStorage {
    fn journal_dir(&self) -> PathBuf;
    fn today_journal_path(&self) -> PathBuf;
    fn open_or_create_today_journal(&self) -> Result<(PathBuf, String)>;
    fn list_journal_entries(&self) -> Result<Vec<JournalEntry>>;
    #[allow(dead_code)]
    fn read_journal_entry(&self, path: &Path) -> Result<String>;
    #[allow(dead_code)]
    fn save_journal_entry(&self, path: &Path, content: &str) -> Result<()>;
}

// TODO: WorkspaceStorage trait defines a complete API for element manipulation.
// Many methods are not yet used by the TUI but are implemented for future features.
// When wiring up element modification commands, these methods will be used.
pub trait WorkspaceStorage {
    fn programs_dir(&self) -> PathBuf;
    fn list_programs(&self) -> Result<Vec<DirectoryEntry>>;
    #[allow(dead_code)]
    fn read_program(&self, name: &str) -> Result<String>;
    #[allow(dead_code)]
    fn save_program(&self, name: &str, content: &str) -> Result<()>;
    #[allow(dead_code)]
    fn create_program(&self, name: &str, description: &str) -> Result<PathBuf>;
    fn list_projects(&self, program: &str) -> Result<Vec<DirectoryEntry>>;
    #[allow(dead_code)]
    fn read_project(&self, program: &str, name: &str) -> Result<String>;
    #[allow(dead_code)]
    fn save_project(&self, program: &str, name: &str, content: &str) -> Result<()>;
    #[allow(dead_code)]
    fn create_project(&self, program: &str, name: &str, description: &str) -> Result<PathBuf>;
    fn list_milestones(&self, program: &str, project: &str) -> Result<Vec<DirectoryEntry>>;
    #[allow(dead_code)]
    fn read_milestone(&self, program: &str, project: &str, name: &str) -> Result<String>;
    #[allow(dead_code)]
    fn save_milestone(&self, program: &str, project: &str, name: &str, content: &str)
        -> Result<()>;
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    fn read_task(
        &self,
        program: &str,
        project: &str,
        milestone: &str,
        name: &str,
    ) -> Result<String>;
    #[allow(dead_code)]
    fn save_task(
        &self,
        program: &str,
        project: &str,
        milestone: &str,
        name: &str,
        content: &str,
    ) -> Result<()>;
    #[allow(dead_code)]
    fn create_task(
        &self,
        program: &str,
        project: &str,
        milestone: &str,
        name: &str,
        description: &str,
    ) -> Result<PathBuf>;
    #[allow(dead_code)]
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
        let path = self.programs_dir().join(format!("{}.md", name));
        Ok(fs::read_to_string(path)?)
    }

    fn save_program(&self, name: &str, content: &str) -> Result<()> {
        let path = self.programs_dir().join(format!("{}.md", name));
        fs::write(path, content)?;
        Ok(())
    }

    fn create_program(&self, name: &str, description: &str) -> Result<PathBuf> {
        let program_md = self.programs_dir().join(format!("{}.md", name));
        let program_dir = self.programs_dir().join(name);
        fs::create_dir_all(&program_dir)?;
        let template = include_str!("../../templates/program.md");
        let truncated_desc = if description.len() > 1024 {
            &description[..1024]
        } else {
            description
        };
        let content = template
            .replace("{{PROGRAM_NAME}}", name)
            .replace("{{DESCRIPTION}}", truncated_desc);
        fs::write(&program_md, content)?;
        Ok(program_md)
    }

    fn list_projects(&self, program: &str) -> Result<Vec<DirectoryEntry>> {
        let program_dir = self.programs_dir().join(program);

        if !program_dir.exists() {
            return Ok(vec![]);
        }

        // The program's own .md file (which we should exclude)
        let program_md_name = format!("{}.md", program);

        let mut entries: Vec<DirectoryEntry> = fs::read_dir(&program_dir)?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                let name = path.file_name()?.to_str()?.to_string();

                // Structure 1: Flat .md file (program/MyProject.md)
                // Exclude the program's own .md file
                if name.ends_with(".md") && name != program_md_name {
                    return Some(DirectoryEntry {
                        name: name.trim_end_matches(".md").to_string(),
                        path,
                        is_dir: false,
                    });
                }

                // Structure 2: Nested in program directory (program/MyProject/MyProject.md)
                if path.is_dir() && name != "projects" {
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

        // Also check for projects in program/projects/ subdirectory (nested structure)
        let projects_subdir = program_dir.join("projects");
        if projects_subdir.exists() {
            let nested_entries: Vec<DirectoryEntry> = fs::read_dir(&projects_subdir)?
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    let path = entry.path();
                    let name = path.file_name()?.to_str()?.to_string();

                    // Nested: program/projects/MyProject/MyProject.md
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
        Ok(entries)
    }

    fn read_project(&self, program: &str, name: &str) -> Result<String> {
        let path = self
            .programs_dir()
            .join(program)
            .join(format!("{}.md", name));
        Ok(fs::read_to_string(path)?)
    }

    fn save_project(&self, program: &str, name: &str, content: &str) -> Result<()> {
        let path = self
            .programs_dir()
            .join(program)
            .join(format!("{}.md", name));
        fs::write(path, content)?;
        Ok(())
    }

    fn create_project(&self, program: &str, name: &str, description: &str) -> Result<PathBuf> {
        let project_md = self
            .programs_dir()
            .join(program)
            .join(format!("{}.md", name));
        let project_dir = self.programs_dir().join(program).join(name);
        fs::create_dir_all(&project_dir)?;
        let template = include_str!("../../templates/project.md");
        let truncated_desc = if description.len() > 1024 {
            &description[..1024]
        } else {
            description
        };
        let content = template
            .replace("{{PROJECT_NAME}}", name)
            .replace("{{DESCRIPTION}}", truncated_desc);
        fs::write(&project_md, content)?;
        Ok(project_md)
    }

    fn list_milestones(&self, program: &str, project: &str) -> Result<Vec<DirectoryEntry>> {
        let mut entries: Vec<DirectoryEntry> = Vec::new();

        // The project's own .md file (which we should exclude)
        let project_md_name = format!("{}.md", project);

        // Check flat structure: programs/program/project/
        let project_dir = self.programs_dir().join(program).join(project);
        if project_dir.exists() {
            let flat_entries: Vec<DirectoryEntry> = fs::read_dir(&project_dir)?
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    let path = entry.path();
                    let name = path.file_name()?.to_str()?.to_string();

                    // Structure 1: Flat .md file (project/MyMilestone.md)
                    // Exclude the project's own .md file
                    if name.ends_with(".md") && name != project_md_name {
                        return Some(DirectoryEntry {
                            name: name.trim_end_matches(".md").to_string(),
                            path,
                            is_dir: false,
                        });
                    }

                    // Structure 2: Nested in project directory (project/MyMilestone/MyMilestone.md)
                    if path.is_dir() && name != "milestones" {
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

            // Check for milestones in project/milestones/ subdirectory
            let milestones_subdir = project_dir.join("milestones");
            if milestones_subdir.exists() {
                let nested_entries: Vec<DirectoryEntry> = fs::read_dir(&milestones_subdir)?
                    .filter_map(|entry| {
                        let entry = entry.ok()?;
                        let path = entry.path();
                        let name = path.file_name()?.to_str()?.to_string();

                        // Nested: project/milestones/MyMilestone/MyMilestone.md
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
        }

        // Check nested structure: programs/program/projects/project/
        let nested_project_dir = self
            .programs_dir()
            .join(program)
            .join("projects")
            .join(project);
        if nested_project_dir.exists() {
            let nested_flat_entries: Vec<DirectoryEntry> = fs::read_dir(&nested_project_dir)?
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    let path = entry.path();
                    let name = path.file_name()?.to_str()?.to_string();

                    // Nested flat: projects/project/MyMilestone.md
                    if name.ends_with(".md") && name != project_md_name {
                        return Some(DirectoryEntry {
                            name: name.trim_end_matches(".md").to_string(),
                            path,
                            is_dir: false,
                        });
                    }

                    // Nested in project directory: projects/project/MyMilestone/MyMilestone.md
                    if path.is_dir() && name != "milestones" {
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
            entries.extend(nested_flat_entries);

            // Check for milestones in projects/project/milestones/ subdirectory
            let nested_milestones_subdir = nested_project_dir.join("milestones");
            if nested_milestones_subdir.exists() {
                let deeply_nested_entries: Vec<DirectoryEntry> =
                    fs::read_dir(&nested_milestones_subdir)?
                        .filter_map(|entry| {
                            let entry = entry.ok()?;
                            let path = entry.path();
                            let name = path.file_name()?.to_str()?.to_string();

                            // Deeply nested: projects/project/milestones/MyMilestone/MyMilestone.md
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
                entries.extend(deeply_nested_entries);
            }
        }

        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    fn read_milestone(&self, program: &str, project: &str, name: &str) -> Result<String> {
        let path = self
            .programs_dir()
            .join(program)
            .join(project)
            .join(format!("{}.md", name));
        Ok(fs::read_to_string(path)?)
    }

    fn save_milestone(
        &self,
        program: &str,
        project: &str,
        name: &str,
        content: &str,
    ) -> Result<()> {
        let path = self
            .programs_dir()
            .join(program)
            .join(project)
            .join(format!("{}.md", name));
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
        let milestone_md = self
            .programs_dir()
            .join(program)
            .join(project)
            .join(format!("{}.md", name));
        let milestone_dir = self.programs_dir().join(program).join(project).join(name);
        fs::create_dir_all(&milestone_dir)?;
        let template = include_str!("../../templates/milestone.md");
        let truncated_desc = if description.len() > 1024 {
            &description[..1024]
        } else {
            description
        };
        let content = template
            .replace("{{MILESTONE_NAME}}", name)
            .replace("{{DESCRIPTION}}", truncated_desc);
        fs::write(&milestone_md, content)?;
        Ok(milestone_md)
    }

    fn list_tasks(
        &self,
        program: &str,
        project: &str,
        milestone: &str,
    ) -> Result<Vec<DirectoryEntry>> {
        let mut entries: Vec<DirectoryEntry> = Vec::new();
        let milestone_md_name = format!("{}.md", milestone);

        // Check flat structure: programs/program/project/milestone/
        let milestone_dir = self
            .programs_dir()
            .join(program)
            .join(project)
            .join(milestone);
        let flat_tasks = discover_elements(&milestone_dir, &milestone_md_name, "tasks")?;
        entries.extend(flat_tasks);

        // Check nested structure: programs/program/projects/project/milestones/milestone/
        let nested_milestone_dir = self
            .programs_dir()
            .join(program)
            .join("projects")
            .join(project)
            .join("milestones")
            .join(milestone);
        let nested_tasks = discover_elements(&nested_milestone_dir, &milestone_md_name, "tasks")?;
        entries.extend(nested_tasks);

        // Deduplicate by name (prefer first occurrence)
        let mut seen = std::collections::HashSet::new();
        entries.retain(|e| seen.insert(e.name.clone()));

        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    fn list_subtasks(
        &self,
        program: &str,
        project: &str,
        milestone: &str,
        task: &str,
    ) -> Result<Vec<DirectoryEntry>> {
        let mut entries: Vec<DirectoryEntry> = Vec::new();
        let task_md_name = format!("{}.md", task);

        // Check flat structure: programs/program/project/milestone/tasks/task/
        let task_dir = self
            .programs_dir()
            .join(program)
            .join(project)
            .join(milestone)
            .join("tasks")
            .join(task);
        let flat_subtasks = discover_elements(&task_dir, &task_md_name, "subtasks")?;
        entries.extend(flat_subtasks);

        // Check nested structure: programs/program/projects/project/milestones/milestone/tasks/task/
        let nested_task_dir = self
            .programs_dir()
            .join(program)
            .join("projects")
            .join(project)
            .join("milestones")
            .join(milestone)
            .join("tasks")
            .join(task);
        let nested_subtasks = discover_elements(&nested_task_dir, &task_md_name, "subtasks")?;
        entries.extend(nested_subtasks);

        // Deduplicate by name (prefer first occurrence)
        let mut seen = std::collections::HashSet::new();
        entries.retain(|e| seen.insert(e.name.clone()));

        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    fn read_task(
        &self,
        program: &str,
        project: &str,
        milestone: &str,
        name: &str,
    ) -> Result<String> {
        let path = self.get_task_path(program, project, milestone, name);
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
        let path = self.get_task_path(program, project, milestone, name);
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
        let task_path = self.get_task_path(program, project, milestone, name);
        if let Some(parent) = task_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let template = include_str!("../../templates/task.md");
        let truncated_desc = if description.len() > 1024 {
            &description[..1024]
        } else {
            description
        };
        let content = template
            .replace("{{TASK_NAME}}", name)
            .replace("{{DESCRIPTION}}", truncated_desc);
        fs::write(&task_path, content)?;
        Ok(task_path)
    }

    fn get_task_path(&self, program: &str, project: &str, milestone: &str, task: &str) -> PathBuf {
        self.programs_dir()
            .join(program)
            .join(project)
            .join(milestone)
            .join(format!("{}.md", task))
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
        let template = match template_name {
            "program" => include_str!("../../templates/program.md"),
            "project" => include_str!("../../templates/project.md"),
            "milestone" => include_str!("../../templates/milestone.md"),
            "task" => include_str!("../../templates/task.md"),
            "subtask" => include_str!("../../templates/subtask.md"),
            _ => return Err(StorageError::TemplateNotFound(template_name.to_string()).into()),
        };

        let content = resolve_template(template, values, strip_labels);

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target_path, content)?;
        Ok(target_path.to_path_buf())
    }
}

pub fn parse_template_fields(template: &str) -> Vec<(String, String, bool)> {
    let mut fields = Vec::new();
    let mut seen_placeholders: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Single pass: scan template line by line to preserve template order
    // Handle different patterns:
    // 1. #Label! {{Placeholder}} or #Label: {{Placeholder}} (on same line)
    // 2. {{Placeholder}} (standalone on its own line)
    // 3. Inline placeholders (e.g., "field: {{VALUE}}" anywhere in line)

    let re_labeled = regex::Regex::new(r"#([^:!]+)![:]?\s*\{\{(\w+)\}\}").unwrap();
    let re_standalone = regex::Regex::new(r"^\{\{(\w+)\}\}$").unwrap();
    let re_inline = regex::Regex::new(r"\{\{(\w+)\}\}").unwrap();

    for line in template.lines() {
        let line_trimmed = line.trim();

        // Check for labeled field pattern: #Label! {{Placeholder}} or #Label: {{Placeholder}}
        if let Some(cap) = re_labeled.captures(line_trimmed) {
            if let (Some(label_match), Some(placeholder_match)) = (cap.get(1), cap.get(2)) {
                let field_label = label_match.as_str().trim().to_string();
                let placeholder = placeholder_match.as_str().to_string();

                if !field_label.is_empty()
                    && !placeholder.is_empty()
                    && !seen_placeholders.contains(&placeholder)
                {
                    let should_strip = field_label.contains('!');
                    let clean_label = field_label.replace('!', "");
                    seen_placeholders.insert(placeholder.clone());
                    fields.push((clean_label, placeholder, should_strip));
                }
            }
            // After processing labeled pattern, continue to next line
            // (don't also match inline placeholder on same line - labeled takes precedence)
            continue;
        }

        // Check for standalone field: {{Placeholder}} (entire line is just the placeholder)
        if let Some(cap) = re_standalone.captures(line_trimmed) {
            if let Some(placeholder_match) = cap.get(1) {
                let placeholder = placeholder_match.as_str().to_string();
                if !placeholder.is_empty() && !seen_placeholders.contains(&placeholder) {
                    // Use placeholder name as label, formatted nicely
                    let label = format_label(&placeholder);
                    seen_placeholders.insert(placeholder.clone());
                    fields.push((label, placeholder, true));
                }
            }
            continue;
        }

        // Check for inline placeholders anywhere in the line
        for cap in re_inline.captures_iter(line_trimmed) {
            if let Some(placeholder_match) = cap.get(1) {
                let placeholder = placeholder_match.as_str().to_string();
                if !placeholder.is_empty() && !seen_placeholders.contains(&placeholder) {
                    let label = format_label(&placeholder);
                    seen_placeholders.insert(placeholder.clone());
                    fields.push((label, placeholder, true));
                }
            }
        }
    }

    fields
}

/// Format a placeholder name as a nice label (e.g., "DUE_DATE" -> "Due Date")
fn format_label(placeholder: &str) -> String {
    placeholder
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
    Ok(entries)
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
}
