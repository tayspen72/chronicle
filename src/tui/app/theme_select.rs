use super::super::{App, Mode};

impl App {
    pub(crate) fn start_theme_selection(&mut self) {
        self.available_themes = crate::theme::loader::list_available_themes();
        tracing::debug!("Available themes: {:?}", self.available_themes);
        if let Some(current) = self
            .available_themes
            .iter()
            .position(|t| t == &self.config.theme)
        {
            self.theme_selection_index = current;
        } else {
            self.theme_selection_index = 0;
        }
        self.previous_theme = Some(self.theme.clone());
        self.mode = Mode::ThemeSelection;
    }

    pub(crate) fn preview_theme(&mut self) {
        if let Some(name) = self
            .available_themes
            .get(self.theme_selection_index)
            .cloned()
        {
            match crate::theme::loader::load_theme(&name) {
                Ok(t) => self.theme = t,
                Err(e) => tracing::warn!("Failed to preview theme '{}': {}", name, e),
            }
        }
    }

    pub(crate) fn confirm_theme_selection(&mut self) {
        if let Some(theme_name) = self
            .available_themes
            .get(self.theme_selection_index)
            .cloned()
        {
            self.config.theme = theme_name.clone();
            if let Err(e) = self.config.save() {
                tracing::error!("Failed to save config: {}", e);
            } else {
                tracing::info!("Confirmed theme: {}", theme_name);
            }
        }
        self.previous_theme = None;
        self.mode = Mode::Normal;
    }

    pub(crate) fn cancel_theme_selection(&mut self) {
        if let Some(prev) = self.previous_theme.take() {
            self.theme = prev;
        }
        self.mode = Mode::Normal;
    }
}
