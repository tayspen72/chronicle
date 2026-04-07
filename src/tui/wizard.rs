//! Wizard state management for element creation.
//!
//! This module contains the wizard state types used for creating
//! programs, projects, milestones, and tasks.

/// Focus state for the template field wizard
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WizardFocus {
    /// Focused on a field at the given index
    Field(usize),
    /// Focused on the CONFIRM button
    ConfirmButton,
    /// Focused on the CANCEL button
    #[default]
    CancelButton,
}

#[derive(Debug, Clone)]
pub enum FieldKind {
    AutoFilled,
    Suggested,
    Empty,
    Fixed,
    Choice,
}

#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub key: String,
    pub label: String,
    pub placeholder: Option<String>,
    pub value: String,
    /// true for user input fields, false for prepopulated keyword fields
    pub is_editable: bool,
    /// true after user edits this field (drives gray -> white value style)
    pub was_edited: bool,
    /// Type of field styling/behavior
    pub kind: FieldKind,
    /// List-backed choices for choice fields
    pub choices: Vec<String>,
    /// Position in template (0-based) to preserve order
    pub display_order: usize,
}

#[derive(Debug, Clone)]
pub struct TemplateFieldState {
    pub template_name: String,
    pub path_hint: String,
    pub fields: Vec<FieldInfo>,
    pub focus: WizardFocus,
    pub values: std::collections::HashMap<String, String>,
    pub strip_labels: std::collections::HashSet<String>,
}

impl TemplateFieldState {
    pub fn new(template_name: String) -> Self {
        Self {
            template_name,
            path_hint: String::new(),
            fields: Vec::new(),
            focus: WizardFocus::default(),
            values: std::collections::HashMap::new(),
            strip_labels: std::collections::HashSet::new(),
        }
    }
}

/// Wizard state wrapper for element creation.
///
/// This wraps TemplateFieldState to allow for future helper methods
/// and follows the extraction pattern used for other App components.
#[derive(Debug, Clone, Default)]
pub struct WizardState {
    pub template: Option<TemplateFieldState>,
}

impl WizardState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_active(&self) -> bool {
        self.template.is_some()
    }

    pub fn start(&mut self, template: TemplateFieldState) {
        self.template = Some(template);
    }

    pub fn clear(&mut self) {
        self.template = None;
    }
}
