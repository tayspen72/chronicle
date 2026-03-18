#[derive(Debug, Clone, Default)]
pub struct AppLifecycle {
    pub should_exit: bool,
    pub needs_terminal_reinit: bool,
}

impl AppLifecycle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn request_exit(&mut self) {
        self.should_exit = true;
    }

    pub fn needs_reinit(&self) -> bool {
        self.needs_terminal_reinit
    }

    pub fn mark_reinit_needed(&mut self) {
        self.needs_terminal_reinit = true;
    }

    pub fn clear_reinit_needed(&mut self) {
        self.needs_terminal_reinit = false;
    }
}
