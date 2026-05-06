use crate::model::SelectedTask;
use crate::storage::md::parse_element;
use crate::storage::{JournalStorage, WorkspaceStorage};
use crate::tui::{DailyTodoItem, PlanningReportKind};

use super::super::{App, Mode};

impl App {
    pub(crate) fn start_review_session(&mut self) {
        if !self.planning_session.active || !self.planning_session.has_tasks() {
            return;
        }
        self.review_state.reset();
        self.planning_session.rolled_over_tasks.clear();
        self.mode = Mode::ReviewSession;
    }

    pub(crate) fn navigate_review(&mut self, direction: isize) {
        if !self.planning_session.has_tasks() {
            return;
        }
        let new_idx = if direction < 0 {
            self.review_state.selection_index.saturating_sub(1)
        } else {
            (self.review_state.selection_index + 1).min(self.planning_session.tasks.len() - 1)
        };
        self.review_state.selection_index = new_idx;
    }

    pub fn backlog_report_tasks(&self) -> Vec<SelectedTask> {
        let snapshots = self.report_task_snapshots();
        let uuids = self
            .backlog_staged_uuids
            .as_ref()
            .cloned()
            .unwrap_or_else(|| self.backlog_task_uuids());
        uuids
            .into_iter()
            .filter_map(|uuid| snapshots.iter().find(|task| task.uuid == uuid).cloned())
            .collect()
    }

    pub fn my_assigned_plan_tasks(&self) -> Vec<SelectedTask> {
        let owner = self.current_user_name().to_ascii_lowercase();
        self.report_task_snapshots()
            .iter()
            .filter(|task| {
                task.assigned_to
                    .as_deref()
                    .is_some_and(|assigned| assigned.trim().to_ascii_lowercase() == owner)
            })
            .cloned()
            .collect()
    }

    pub fn today_todo_items(&self) -> Vec<DailyTodoItem> {
        let path = self.config.workspace.today_journal_path();
        let Ok(content) = self.config.workspace.read_md_file(&path) else {
            return Vec::new();
        };
        Self::parse_todo_items(&content)
    }

    pub(crate) fn current_user_name(&self) -> String {
        let owner = self.config.owner.trim();
        if !owner.is_empty() {
            owner.to_string()
        } else {
            std::env::var("USER")
                .ok()
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| "me".to_string())
        }
    }

    pub(crate) fn report_task_snapshots(&self) -> Vec<SelectedTask> {
        self.planning_session
            .tasks
            .iter()
            .map(|task| self.task_snapshot_from_file(task))
            .collect()
    }

    pub(crate) fn task_snapshot_from_file(&self, task: &SelectedTask) -> SelectedTask {
        let mut snapshot = task.clone();
        let Ok(content) = self.config.workspace.read_md_file(&task.path) else {
            return snapshot;
        };
        let Ok(Some(element)) = parse_element(&content) else {
            return snapshot;
        };
        let crate::model::Element::Task(parsed_task) = element else {
            return snapshot;
        };
        snapshot.task_name = parsed_task.title;
        snapshot.status = parsed_task.status;
        snapshot.assigned_to = parsed_task.assigned_to.clone();
        snapshot.start_date = parsed_task.start_date.clone();
        snapshot.due_date = parsed_task.due_date.clone();
        snapshot.importance = parsed_task.importance.clone();
        snapshot.description = Some(parsed_task.description.clone());
        snapshot
    }

    pub(crate) fn backlog_task_uuids(&self) -> Vec<String> {
        self.planning_session
            .tasks
            .iter()
            .filter(|task| {
                task.assigned_to
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
            })
            .map(|task| task.uuid.clone())
            .collect()
    }

    pub(crate) fn backlog_task_indices_in_session(&self) -> Vec<usize> {
        let uuids = self
            .backlog_staged_uuids
            .as_ref()
            .cloned()
            .unwrap_or_else(|| self.backlog_task_uuids());
        uuids
            .into_iter()
            .filter_map(|uuid| {
                self.planning_session
                    .tasks
                    .iter()
                    .position(|task| task.uuid == uuid)
            })
            .collect()
    }

    pub(crate) fn my_plan_task_indices_in_session(&self) -> Vec<usize> {
        let owner = self.current_user_name().to_ascii_lowercase();
        self.planning_session
            .tasks
            .iter()
            .enumerate()
            .filter_map(|(idx, task)| {
                task.assigned_to
                    .as_deref()
                    .is_some_and(|assigned| assigned.trim().to_ascii_lowercase() == owner)
                    .then_some(idx)
            })
            .collect()
    }

    pub(crate) fn navigate_report_rows(&mut self, direction: isize) {
        let count = match self.active_planning_report() {
            Some(PlanningReportKind::Backlog) => self.backlog_task_indices_in_session().len(),
            Some(PlanningReportKind::MyTasks) => {
                self.my_plan_task_indices_in_session().len() + self.today_todo_items().len()
            }
            _ => 0,
        };
        if count == 0 {
            self.review_state.selection_index = 0;
            return;
        }

        let current = self.review_state.selection_index.min(count - 1);
        let new_idx = if direction < 0 {
            current.saturating_sub(1)
        } else {
            (current + 1).min(count - 1)
        };
        self.review_state.selection_index = new_idx;
    }

    pub(crate) fn assign_selected_backlog_task_to_owner(&mut self) {
        let report_idx = self.review_state.selection_index;
        let indices = self.backlog_task_indices_in_session();
        let Some(&task_idx) = indices.get(self.review_state.selection_index) else {
            return;
        };
        self.review_state.selection_index = task_idx;
        self.set_task_assigned_to(self.current_user_name());
        self.save_current_planning_session();
        self.review_state.selection_index = report_idx.min(indices.len().saturating_sub(1));
    }

    pub(crate) fn cycle_selected_my_task_status(&mut self) {
        let report_idx = self.review_state.selection_index;
        let task_indices = self.my_plan_task_indices_in_session();
        let Some(&task_idx) = task_indices.get(self.review_state.selection_index) else {
            return;
        };
        self.review_state.selection_index = task_idx;
        self.cycle_task_status();
        self.review_state.selection_index = report_idx.min(task_indices.len().saturating_sub(1));
    }

    pub(crate) fn toggle_selected_my_todo(&mut self) {
        let task_count = self.my_plan_task_indices_in_session().len();
        if self.review_state.selection_index < task_count {
            return;
        }
        let todo_idx = self.review_state.selection_index - task_count;
        let mut lines;
        let path = self.config.workspace.today_journal_path();
        let Ok(content) = self.config.workspace.read_md_file(&path) else {
            return;
        };
        lines = content.lines().map(ToString::to_string).collect::<Vec<_>>();
        let ends_with_newline = content.ends_with('\n');

        let todos = Self::parse_todo_items(&content);
        let Some(todo) = todos.get(todo_idx) else {
            return;
        };
        if todo.line_index >= lines.len() {
            return;
        }

        let marker = if todo.checked { ' ' } else { 'x' };
        lines[todo.line_index] = format!("- [{}] {}", marker, todo.text);
        let mut updated = lines.join("\n");
        if ends_with_newline {
            updated.push('\n');
        }
        if let Err(e) = self.config.workspace.save_journal_entry(&path, &updated) {
            eprintln!("Failed to update today's To Do item: {e}");
        }
    }

    pub(crate) fn clear_backlog_staging_if_needed(&mut self) {
        if self.active_planning_report() != Some(PlanningReportKind::Backlog) {
            self.backlog_staged_uuids = None;
        }
    }

    pub(crate) fn parse_todo_items(content: &str) -> Vec<DailyTodoItem> {
        let mut items = Vec::new();
        let mut in_todo = false;

        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                let heading = trimmed.trim_start_matches('#').trim().to_ascii_lowercase();
                if heading == "to do" || heading == "todo" || heading == "to-do" {
                    in_todo = true;
                    continue;
                }
                if in_todo {
                    break;
                }
                continue;
            }

            if !in_todo {
                continue;
            }

            if let Some(text) = trimmed
                .strip_prefix("- [ ] ")
                .or(trimmed.strip_prefix("* [ ] "))
            {
                items.push(DailyTodoItem {
                    line_index: idx,
                    checked: false,
                    text: text.trim().to_string(),
                });
            } else if let Some(text) = trimmed
                .strip_prefix("- [x] ")
                .or(trimmed.strip_prefix("* [x] "))
                .or(trimmed.strip_prefix("- [X] "))
                .or(trimmed.strip_prefix("* [X] "))
            {
                items.push(DailyTodoItem {
                    line_index: idx,
                    checked: true,
                    text: text.trim().to_string(),
                });
            }
        }

        items
    }

    pub(crate) fn open_current_plan_task_in_editor(&mut self) {
        let Some(task) = self
            .planning_session
            .tasks
            .get(self.review_state.selection_index)
            .cloned()
        else {
            return;
        };
        self.launch_editor(&task.path);
    }

    pub(crate) fn cycle_task_status(&mut self) {
        let Some(task) = self
            .planning_session
            .tasks
            .get(self.review_state.selection_index)
        else {
            return;
        };
        let workflow = &self.config.workflow;
        if workflow.is_empty() {
            return;
        }
        let current_idx = workflow.iter().position(|s| s == &task.status).unwrap_or(0);
        let next_idx = (current_idx + 1) % workflow.len();
        let new_status = workflow[next_idx].clone();
        self.update_review_task_status(&new_status);
    }

    pub(crate) fn set_task_start_date(&mut self, date: String) {
        let Some(task) = self
            .planning_session
            .tasks
            .get_mut(self.review_state.selection_index)
        else {
            return;
        };
        task.start_date = Some(date.clone());
        if let Err(e) = crate::storage::md::update_task_fields(
            &task.path,
            [("start_date", Some(date))].into_iter().collect(),
        ) {
            eprintln!("Failed to update task start date: {e}");
        }
    }

    pub(crate) fn set_task_due_date(&mut self, date: String) {
        let Some(task) = self
            .planning_session
            .tasks
            .get_mut(self.review_state.selection_index)
        else {
            return;
        };
        task.due_date = Some(date.clone());
        if let Err(e) = crate::storage::md::update_task_fields(
            &task.path,
            [("due_date", Some(date))].into_iter().collect(),
        ) {
            eprintln!("Failed to update task due date: {e}");
        }
    }

    pub(crate) fn set_task_assigned_to(&mut self, name: String) {
        let idx = self.review_state.selection_index;
        let Some(task_path) = self
            .planning_session
            .tasks
            .get(idx)
            .map(|task| task.path.clone())
        else {
            return;
        };
        let assigned_value = name.clone();
        if let Err(e) = crate::storage::md::update_task_fields(
            &task_path,
            [("assigned_to", Some(name))].into_iter().collect(),
        ) {
            eprintln!("Failed to update task assigned_to: {e}");
            return;
        }
        if let Some(task) = self.planning_session.tasks.get_mut(idx) {
            task.assigned_to = Some(assigned_value);
        }
    }

    pub(crate) fn mark_task_done(&mut self) {
        self.update_review_task_status("done");
    }

    pub(crate) fn update_review_task_status(&mut self, new_status: &str) {
        let Some(task) = self
            .planning_session
            .tasks
            .get_mut(self.review_state.selection_index)
        else {
            return;
        };
        let old_status = task.status.clone();
        if old_status == new_status {
            return;
        }

        if let Err(e) = crate::storage::md::update_task_status(&task.path, new_status) {
            eprintln!("Failed to update task status: {e}");
            return;
        }

        task.status = new_status.to_string();

        self.save_current_planning_session();
    }

    pub(crate) fn toggle_rollover(&mut self) {
        let Some(task) = self
            .planning_session
            .tasks
            .get(self.review_state.selection_index)
        else {
            return;
        };
        let uuid = &task.uuid;

        if let Some(pos) = self
            .planning_session
            .rolled_over_tasks
            .iter()
            .position(|u| u == uuid)
        {
            self.planning_session.rolled_over_tasks.remove(pos);
        } else {
            self.planning_session.rolled_over_tasks.push(uuid.clone());
        }
    }

    pub(crate) fn remove_task_from_session(&mut self) {
        if !self.planning_session.has_tasks() {
            return;
        }
        let Some(task) = self
            .planning_session
            .tasks
            .get(self.review_state.selection_index)
        else {
            return;
        };

        if let Some(pos) = self
            .planning_session
            .rolled_over_tasks
            .iter()
            .position(|u| u == &task.uuid)
        {
            self.planning_session.rolled_over_tasks.remove(pos);
        }

        self.planning_session
            .tasks
            .remove(self.review_state.selection_index);

        if self.review_state.selection_index >= self.planning_session.tasks.len()
            && self.review_state.selection_index > 0
        {
            self.review_state.selection_index -= 1;
        }

        self.save_current_planning_session();
    }
}
