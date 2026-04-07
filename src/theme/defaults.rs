use crate::theme::{
    BorderScopes, CommandScopes, ContentScopes, SelectionScopes, SidebarScopes, StatusScopes,
    TextScopes, Theme, UiScopes, WizardScopes,
};
use ratatui::style::{Color, Modifier, Style};
use std::collections::HashMap;

pub fn default_theme() -> Theme {
    Theme {
        palette: default_palette(),
        ui: default_ui_scopes(),
    }
}

fn default_palette() -> HashMap<String, Color> {
    let mut palette = HashMap::new();
    palette.insert("black".to_string(), Color::Black);
    palette.insert("white".to_string(), Color::White);
    palette.insert("dark_gray".to_string(), Color::DarkGray);
    palette.insert("gray".to_string(), Color::Gray);
    palette.insert("light_gray".to_string(), Color::Gray);
    palette.insert("blue".to_string(), Color::Blue);
    palette.insert("light_blue".to_string(), Color::LightBlue);
    palette.insert("cyan".to_string(), Color::Cyan);
    palette.insert("light_cyan".to_string(), Color::LightCyan);
    palette.insert("yellow".to_string(), Color::Yellow);
    palette.insert("light_yellow".to_string(), Color::LightYellow);
    palette.insert("green".to_string(), Color::Green);
    palette.insert("magenta".to_string(), Color::Magenta);
    palette.insert("light_magenta".to_string(), Color::LightMagenta);
    palette
}

fn default_ui_scopes() -> UiScopes {
    UiScopes {
        background: Some(Style::default().bg(Color::Black)),
        surface: Some(Style::default().bg(Color::DarkGray)),
        text: default_text_scopes(),
        selection: default_selection_scopes(),
        border: default_border_scopes(),
        sidebar: default_sidebar_scopes(),
        status: default_status_scopes(),
        content: default_content_scopes(),
        wizard: default_wizard_scopes(),
        command: default_command_scopes(),
    }
}

fn default_text_scopes() -> TextScopes {
    TextScopes {
        primary: Some(Style::default().fg(Color::White)),
        secondary: Some(Style::default().fg(Color::DarkGray)),
        muted: Some(Style::default().fg(Color::DarkGray)),
    }
}

fn default_selection_scopes() -> SelectionScopes {
    SelectionScopes {
        bg: Some(Color::LightBlue),
        fg: Some(Color::Black),
        active: Some(Style::default().fg(Color::Black).bg(Color::Yellow)),
    }
}

fn default_border_scopes() -> BorderScopes {
    BorderScopes {
        normal: Some(Style::default().fg(Color::DarkGray)),
        focused: Some(Style::default().fg(Color::LightBlue)),
    }
}

fn default_sidebar_scopes() -> SidebarScopes {
    SidebarScopes {
        header: Some(Style::default().fg(Color::DarkGray)),
        item: Some(Style::default().fg(Color::White)),
        selected: Some(
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),
        ),
        create_action: Some(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::ITALIC),
        ),
    }
}

fn default_status_scopes() -> StatusScopes {
    StatusScopes {
        normal: Some(Color::Green),
        command: Some(Color::Yellow),
        input: Some(Color::Cyan),
        select: Some(Color::Magenta),
        review: Some(Color::LightMagenta),
        hierarchical_selection: Some(Color::LightCyan),
        planning_preview: Some(Color::LightBlue),
        task_detail_wizard: Some(Color::LightYellow),
        current_plan_navigation: Some(Color::LightCyan),
        theme_selection: Some(Color::LightCyan),
    }
}

fn default_content_scopes() -> ContentScopes {
    ContentScopes {
        title: Some(
            Style::default()
                .fg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),
        ),
        header: Some(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        table_header: Some(Style::default().fg(Color::LightBlue)),
        table_border: Some(Style::default().fg(Color::DarkGray)),
    }
}

fn default_wizard_scopes() -> WizardScopes {
    WizardScopes {
        field_label: Some(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        field_value: Some(Style::default().fg(Color::White)),
        field_value_empty: Some(Style::default().fg(Color::DarkGray)),
        field_value_auto: Some(Style::default().fg(Color::DarkGray)),
        button_confirm: Some(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        button_cancel: Some(Style::default().fg(Color::DarkGray)),
        button_selected: Some(Style::default().fg(Color::Black).bg(Color::LightBlue)),
    }
}

fn default_command_scopes() -> CommandScopes {
    CommandScopes {
        input: Some(Style::default().fg(Color::White).bg(Color::Black)),
        result: Some(Style::default().fg(Color::White).bg(Color::Black)),
        result_selected: Some(
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),
        ),
    }
}
