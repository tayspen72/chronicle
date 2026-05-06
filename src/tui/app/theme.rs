//! Theme/style accessors on `App`. These are pure functions over
//! `self.theme` — moved out of `tui/mod.rs` to keep the main file focused
//! on event handling and state transitions.

use ratatui::style::Style;

use super::super::{App, Mode};

impl App {
    pub fn background_style(&self) -> ratatui::style::Style {
        self.theme.ui.background.unwrap_or_default()
    }

    pub fn text_primary(&self) -> ratatui::style::Style {
        self.theme
            .ui
            .text
            .primary
            .unwrap_or_else(|| ratatui::style::Style::default().fg(ratatui::style::Color::White))
    }

    pub fn text_secondary(&self) -> ratatui::style::Style {
        self.theme
            .ui
            .text
            .secondary
            .unwrap_or_else(|| ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray))
    }

    pub fn sidebar_style(&self) -> ratatui::style::Style {
        self.theme.ui.sidebar.item.unwrap_or_default()
    }

    pub fn sidebar_selected_style(&self) -> ratatui::style::Style {
        self.theme.ui.sidebar.selected.unwrap_or_else(|| {
            ratatui::style::Style::default()
                .fg(ratatui::style::Color::Black)
                .bg(ratatui::style::Color::LightBlue)
                .add_modifier(ratatui::style::Modifier::BOLD)
        })
    }

    pub fn sidebar_header_style(&self) -> ratatui::style::Style {
        self.theme
            .ui
            .sidebar
            .header
            .unwrap_or_else(|| ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray))
    }

    pub fn sidebar_create_action_style(&self, selected: bool) -> ratatui::style::Style {
        if let Some(style) = &self.theme.ui.sidebar.create_action {
            if selected {
                return (*style)
                    .bg(ratatui::style::Color::Cyan)
                    .fg(ratatui::style::Color::Black);
            }
            return *style;
        }
        if selected {
            ratatui::style::Style::default()
                .fg(ratatui::style::Color::Black)
                .bg(ratatui::style::Color::Cyan)
        } else {
            ratatui::style::Style::default().fg(ratatui::style::Color::Cyan)
        }
    }

    pub fn border_style(&self) -> ratatui::style::Style {
        self.theme
            .ui
            .border
            .normal
            .unwrap_or_else(|| ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray))
    }

    pub fn status_color(&self) -> ratatui::style::Color {
        match self.mode {
            Mode::Normal => self
                .theme
                .ui
                .status
                .normal
                .unwrap_or(ratatui::style::Color::Green),
            Mode::CommandPalette => self
                .theme
                .ui
                .status
                .command
                .unwrap_or(ratatui::style::Color::Yellow),
            Mode::Input | Mode::InputTaskDetailField => self
                .theme
                .ui
                .status
                .input
                .unwrap_or(ratatui::style::Color::Cyan),
            Mode::TaskSelection => self
                .theme
                .ui
                .status
                .select
                .unwrap_or(ratatui::style::Color::Magenta),
            Mode::ReviewSession => self
                .theme
                .ui
                .status
                .review
                .unwrap_or(ratatui::style::Color::LightMagenta),
            Mode::HierarchicalSelection => self
                .theme
                .ui
                .status
                .hierarchical_selection
                .unwrap_or(ratatui::style::Color::LightCyan),
            Mode::PlanningPreview => self
                .theme
                .ui
                .status
                .planning_preview
                .unwrap_or(ratatui::style::Color::LightBlue),
            Mode::TaskDetailWizard => self
                .theme
                .ui
                .status
                .task_detail_wizard
                .unwrap_or(ratatui::style::Color::LightYellow),
            Mode::CurrentPlanNavigation => self
                .theme
                .ui
                .status
                .current_plan_navigation
                .unwrap_or(ratatui::style::Color::LightCyan),
            Mode::ThemeSelection => self
                .theme
                .ui
                .status
                .theme_selection
                .unwrap_or(ratatui::style::Color::LightCyan),
        }
    }

    pub fn command_input_style(&self) -> ratatui::style::Style {
        self.theme.ui.command.input.unwrap_or_else(|| {
            ratatui::style::Style::default()
                .fg(ratatui::style::Color::White)
                .bg(ratatui::style::Color::Black)
        })
    }

    pub fn command_result_style(&self) -> ratatui::style::Style {
        self.theme.ui.command.result.unwrap_or_else(|| {
            ratatui::style::Style::default()
                .fg(ratatui::style::Color::White)
                .bg(ratatui::style::Color::Black)
        })
    }

    pub fn command_result_selected_style(&self) -> ratatui::style::Style {
        self.theme.ui.command.result_selected.unwrap_or_else(|| {
            ratatui::style::Style::default()
                .fg(ratatui::style::Color::Black)
                .bg(ratatui::style::Color::LightBlue)
                .add_modifier(ratatui::style::Modifier::BOLD)
        })
    }

    pub fn command_section_style(&self) -> ratatui::style::Style {
        self.text_secondary()
            .add_modifier(ratatui::style::Modifier::BOLD)
    }

    pub fn command_border_style(&self) -> ratatui::style::Style {
        self.theme.ui.border.focused.unwrap_or_else(|| {
            ratatui::style::Style::default().fg(ratatui::style::Color::LightBlue)
        })
    }

    pub fn theme_border_style(&self) -> ratatui::style::Style {
        self.theme.ui.border.focused.unwrap_or_else(|| {
            ratatui::style::Style::default().fg(ratatui::style::Color::LightCyan)
        })
    }

    pub fn content_title_style(&self) -> ratatui::style::Style {
        self.theme.ui.content.title.unwrap_or_else(|| {
            ratatui::style::Style::default()
                .fg(ratatui::style::Color::LightBlue)
                .add_modifier(ratatui::style::Modifier::BOLD)
        })
    }

    pub fn content_header_style(&self) -> ratatui::style::Style {
        self.theme.ui.content.header.unwrap_or_else(|| {
            ratatui::style::Style::default()
                .fg(ratatui::style::Color::White)
                .add_modifier(ratatui::style::Modifier::BOLD)
        })
    }

    pub fn content_table_header_style(&self) -> ratatui::style::Style {
        self.theme.ui.content.table_header.unwrap_or_else(|| {
            ratatui::style::Style::default().fg(ratatui::style::Color::LightBlue)
        })
    }

    pub fn content_table_border_style(&self) -> ratatui::style::Style {
        self.theme
            .ui
            .content
            .table_border
            .unwrap_or_else(|| ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray))
    }

    pub fn wizard_field_label_style(&self) -> ratatui::style::Style {
        self.theme.ui.wizard.field_label.unwrap_or_else(|| {
            ratatui::style::Style::default()
                .fg(ratatui::style::Color::White)
                .add_modifier(ratatui::style::Modifier::BOLD)
        })
    }

    pub fn wizard_field_value_style(&self) -> ratatui::style::Style {
        self.theme
            .ui
            .wizard
            .field_value
            .unwrap_or_else(|| ratatui::style::Style::default().fg(ratatui::style::Color::White))
    }

    pub fn wizard_field_empty_style(&self) -> ratatui::style::Style {
        self.theme
            .ui
            .wizard
            .field_value_empty
            .unwrap_or_else(|| ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray))
    }

    pub fn wizard_field_auto_style(&self) -> ratatui::style::Style {
        self.theme
            .ui
            .wizard
            .field_value_auto
            .unwrap_or_else(|| ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray))
    }

    pub fn wizard_button_confirm_style(&self) -> ratatui::style::Style {
        self.theme.ui.wizard.button_confirm.unwrap_or_else(|| {
            ratatui::style::Style::default()
                .fg(ratatui::style::Color::Black)
                .bg(ratatui::style::Color::Cyan)
                .add_modifier(ratatui::style::Modifier::BOLD)
        })
    }

    pub fn wizard_button_cancel_style(&self) -> ratatui::style::Style {
        self.theme
            .ui
            .wizard
            .button_cancel
            .unwrap_or_else(|| ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray))
    }

    pub fn wizard_button_selected_style(&self) -> ratatui::style::Style {
        self.theme.ui.wizard.button_selected.unwrap_or_else(|| {
            ratatui::style::Style::default()
                .fg(ratatui::style::Color::Black)
                .bg(ratatui::style::Color::LightBlue)
        })
    }

    pub fn selection_bg(&self) -> ratatui::style::Color {
        self.theme
            .ui
            .selection
            .bg
            .unwrap_or(ratatui::style::Color::LightBlue)
    }

    pub fn selection_fg(&self) -> ratatui::style::Style {
        if let Some(fg) = self.theme.ui.selection.fg {
            Style::default().fg(fg)
        } else {
            Style::default().fg(ratatui::style::Color::Black)
        }
    }

    pub fn hierarchy_program_style(&self) -> ratatui::style::Style {
        Style::default().add_modifier(ratatui::style::Modifier::BOLD)
    }

    pub fn hierarchy_project_style(&self) -> ratatui::style::Style {
        Style::default().add_modifier(ratatui::style::Modifier::ITALIC)
    }

    pub fn hierarchy_milestone_style(&self) -> ratatui::style::Style {
        Style::default().fg(ratatui::style::Color::LightCyan)
    }

    pub fn selection_active_style(&self) -> ratatui::style::Style {
        self.theme.ui.selection.active.unwrap_or_else(|| {
            Style::default()
                .fg(ratatui::style::Color::Black)
                .bg(ratatui::style::Color::Yellow)
        })
    }
}
