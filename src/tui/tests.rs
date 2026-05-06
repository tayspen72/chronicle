use super::*;

#[test]
fn test_initial_mode_is_normal() {
    let config = crate::config::Config::default();
    let app = App::new(config);
    assert!(matches!(app.mode, Mode::Normal));
}

#[test]
fn test_slash_enters_command_palette_mode() {
    let config = crate::config::Config::default();
    let mut app = App::new(config);

    // Simulate pressing '/'
    app.handle_key(KeyCode::Char('/'));

    assert!(matches!(app.mode, Mode::CommandPalette));
}

#[test]
fn test_esc_exits_command_palette_mode() {
    let config = crate::config::Config::default();
    let mut app = App::new(config);

    // First enter command palette mode
    app.handle_key(KeyCode::Char('/'));
    assert!(matches!(app.mode, Mode::CommandPalette));

    // Then press Esc to exit (from command input handler since mode is CommandPalette)
    app.handle_command_input(KeyCode::Esc);

    assert!(matches!(app.mode, Mode::Normal));
}

#[test]
fn test_enter_returns_to_normal_mode() {
    let config = crate::config::Config::default();
    let mut app = App::new(config);

    // Enter command palette mode
    app.handle_key(KeyCode::Char('/'));
    assert!(matches!(app.mode, Mode::CommandPalette));

    // Press Enter (should execute command and return to normal)
    app.handle_command_input(KeyCode::Enter);

    assert!(matches!(app.mode, Mode::Normal));
}

#[test]
fn test_command_palette_has_new_program_with_empty_workspace() {
    // Simulate an empty workspace scenario
    let mut config = crate::config::Config::default();
    // Set workspace to a temp directory (simulating empty workspace)
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    config.workspace = temp_dir.path().to_path_buf();

    let mut app = App::new(config);

    // Verify programs list is empty
    assert!(
        app.tree_data.programs.is_empty(),
        "Programs should be empty in new workspace"
    );

    // Verify sidebar has items (Planning and Journal sections should exist)
    assert!(
        !app.navigation_state.sidebar_items.is_empty(),
        "Sidebar should have items even with empty programs"
    );

    // Open command palette
    app.handle_key(KeyCode::Char('/'));
    assert!(matches!(app.mode, Mode::CommandPalette));

    // Verify "New Program" is in the command list
    assert!(
        app.command_palette
            .matches
            .iter()
            .any(|c| c.label == "New Program"),
        "New Program command should be available even with empty workspace"
    );

    // Verify we can navigate the command list
    assert!(
        !app.command_palette.matches.is_empty(),
        "Command list should not be empty"
    );

    // Verify we can select "New Program" command
    let new_program_idx = app
        .command_palette
        .matches
        .iter()
        .position(|c| c.label == "New Program");
    assert!(
        new_program_idx.is_some(),
        "Should be able to find New Program command index"
    );

    // Navigate to New program command
    if let Some(idx) = new_program_idx {
        app.command_palette.selection_index = idx;
        assert_eq!(app.command_palette.matches[idx].label, "New Program");
    }
}

#[test]
fn test_inline_editing_updates_field_value() {
    let config = crate::config::Config::default();
    let mut app = App::new(config);

    // Set up template field state for testing
    let fields = vec![
        FieldInfo {
            key: "title".to_string(),
            label: "Title".to_string(),
            placeholder: Some("TITLE".to_string()),
            value: String::new(),
            is_editable: true,
            was_edited: false,
            kind: FieldKind::Empty,
            choices: Vec::new(),
            display_order: 0,
        },
        FieldInfo {
            key: "status".to_string(),
            label: "Status".to_string(),
            placeholder: Some("DEFAULT_STATUS".to_string()),
            value: "New".to_string(),
            is_editable: false,
            was_edited: false,
            kind: FieldKind::AutoFilled,
            choices: Vec::new(),
            display_order: 1,
        },
    ];

    app.wizard_state.template = Some(TemplateFieldState {
        template_name: "test".to_string(),
        path_hint: "Programs -> new program".to_string(),
        fields,
        focus: WizardFocus::Field(0),
        values: std::collections::HashMap::new(),
        strip_labels: std::collections::HashSet::new(),
    });
    app.current_view = ViewType::InputTemplateField;
    app.input_buffer.clear();

    // Type some characters
    app.handle_key(KeyCode::Char('H'));
    app.handle_key(KeyCode::Char('e'));
    app.handle_key(KeyCode::Char('l'));
    app.handle_key(KeyCode::Char('l'));
    app.handle_key(KeyCode::Char('o'));

    // Verify both input_buffer and field.value are updated
    assert_eq!(app.input_buffer, "Hello");
    if let Some(state) = &app.wizard_state.template {
        assert_eq!(state.fields[0].value, "Hello");
    }

    // Test backspace
    app.handle_key(KeyCode::Backspace);
    assert_eq!(app.input_buffer, "Hell");
    if let Some(state) = &app.wizard_state.template {
        assert_eq!(state.fields[0].value, "Hell");
    }
}

#[test]
fn test_navigator_refreshes_after_creating_element() {
    // Create a temp workspace
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let workspace_path = temp_dir.path().to_path_buf();

    // Create a program directory manually so we have something to work with
    let programs_dir = workspace_path.join("programs");
    std::fs::create_dir_all(&programs_dir).expect("Failed to create programs dir");

    // Create a simple program file
    let program_content = r#"---
uuid: test-uuid
title: TestProgram
status: New
tags: program
---

# DESCRIPTION
Test description
"#;
    std::fs::write(programs_dir.join("TestProgram.md"), program_content)
        .expect("Failed to create program file");

    // Set up config with temp workspace
    let config = crate::config::Config {
        workspace: workspace_path.clone(),
        ..crate::config::Config::default()
    };

    let mut app = App::new(config);

    // Verify we're at root level with programs loaded
    assert!(
        !app.tree_data.programs.is_empty(),
        "Programs should be loaded"
    );
    assert_eq!(
        app.navigation_state.sidebar_tree.selected_depth(),
        0,
        "Should be at root level"
    );

    // Navigate into the program (select index 1 because index 0 is "Programs" header)
    app.navigation_state.selected_entry_index = 1;
    app.open_tree_item();

    // Verify we're now inside the program
    assert_eq!(
        app.navigation_state.sidebar_tree.selected_depth(),
        1,
        "Should be inside program"
    );
    assert!(
        app.navigation_state.current_program.is_some(),
        "Current program should be set"
    );

    // Now simulate creating a new project via the wizard
    // First, start the new project wizard
    app.start_new_project();

    // Verify we're in template field input mode
    assert_eq!(app.current_view, ViewType::InputTemplateField);
    assert!(app.wizard_state.template.is_some());

    // Fill in the project name in the first editable field
    if let Some(ref mut state) = app.wizard_state.template {
        // Find first editable field and set its value
        for field in &mut state.fields {
            if field.is_editable {
                field.value = "NewProject".to_string();
                break;
            }
        }
        // Set focus to ConfirmButton to simulate pressing tab through all fields
        state.focus = WizardFocus::ConfirmButton;
    }
    app.input_buffer = "NewProject".to_string();

    // Confirm the creation
    app.confirm_template_field();

    // After creation, verify the navigator was refreshed
    // NEW BEHAVIOR: Stay at parent level (don't auto-navigate into new element)
    // The user can manually navigate into it with arrow key
    assert_eq!(
        app.navigation_state.sidebar_tree.selected_depth(),
        1,
        "Should stay at parent level (program) after creation"
    );
    assert!(
        app.navigation_state.current_project.is_none(),
        "Should NOT auto-navigate into project - current_project should be None"
    );

    // Verify we're back in TreeView
    assert_eq!(app.current_view, ViewType::TreeView);

    // The key test: verify that sidebar_items reflects the current state
    // After creating a project, we should still be in the program, showing projects
    assert!(
        !app.navigation_state.sidebar_items.is_empty(),
        "Sidebar should have items after creation"
    );

    // Selected nodes keep children collapsed, so project children are not shown here.
    let has_new_project = app
        .navigation_state
        .sidebar_items
        .iter()
        .any(|item| !item.is_header && item.name == "NewProject");
    assert!(
        !has_new_project,
        "Selected program should keep project children collapsed"
    );
}

#[test]
fn test_wizard_creates_program_file_on_disk() {
    // Create a temp workspace
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let workspace_path = temp_dir.path().to_path_buf();

    // Set up config with temp workspace
    let config = crate::config::Config {
        workspace: workspace_path.clone(),
        ..crate::config::Config::default()
    };

    let mut app = App::new(config);

    // Start the new program wizard
    app.start_new_program();

    // Verify we're in template field input mode
    assert_eq!(app.current_view, ViewType::InputTemplateField);
    assert!(app.wizard_state.template.is_some());

    // Fill in the program name in the first editable field
    if let Some(ref mut state) = app.wizard_state.template {
        // Find first editable field and set its value
        for field in &mut state.fields {
            if field.is_editable {
                field.value = "MyNewProgram".to_string();
                break;
            }
        }
        // Set focus to ConfirmButton
        state.focus = WizardFocus::ConfirmButton;
    }
    app.input_buffer = "MyNewProgram".to_string();

    // Confirm the creation
    app.confirm_template_field();

    // Verify the file was created on disk
    let program_path = workspace_path
        .join("programs")
        .join("MyNewProgram")
        .join("MyNewProgram.md");
    assert!(
        program_path.exists(),
        "Program file should be created at {:?}",
        program_path
    );

    // Verify the file has content from the template
    let content = std::fs::read_to_string(&program_path).expect("Should be able to read file");
    assert!(
        content.contains("MyNewProgram"),
        "Program file should contain the name"
    );
}

#[test]
fn test_wizard_creates_project_file_on_disk() {
    // Create a temp workspace
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let workspace_path = temp_dir.path().to_path_buf();

    // Create a program directory manually so we have something to work with
    let programs_dir = workspace_path.join("programs");
    std::fs::create_dir_all(&programs_dir).expect("Failed to create programs dir");

    // Create a simple program file
    let program_content = r#"---
uuid: test-uuid
title: TestProgram
status: New
tags: program
---

# DESCRIPTION
Test description
"#;
    std::fs::write(programs_dir.join("TestProgram.md"), program_content)
        .expect("Failed to create program file");

    // Set up config with temp workspace
    let config = crate::config::Config {
        workspace: workspace_path.clone(),
        ..crate::config::Config::default()
    };

    let mut app = App::new(config);

    // Navigate into the program
    app.navigation_state.selected_entry_index = 1;
    app.open_tree_item();

    // Start the new project wizard
    app.start_new_project();

    // Fill in the project name
    if let Some(ref mut state) = app.wizard_state.template {
        for field in &mut state.fields {
            if field.is_editable {
                field.value = "NewProject".to_string();
                break;
            }
        }
        state.focus = WizardFocus::ConfirmButton;
    }
    app.input_buffer = "NewProject".to_string();

    // Confirm the creation
    app.confirm_template_field();

    // Verify the file was created on disk
    let project_path = workspace_path
        .join("programs")
        .join("TestProgram")
        .join("projects")
        .join("NewProject")
        .join("NewProject.md");
    assert!(
        project_path.exists(),
        "Project file should be created at {:?}",
        project_path
    );

    // Verify the file has content from the template
    let content = std::fs::read_to_string(&project_path).expect("Should be able to read file");
    assert!(
        content.contains("NewProject"),
        "Project file should contain the name"
    );
}

#[test]
fn test_wizard_creates_milestone_file_on_disk() {
    // Create a temp workspace with program and project
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let workspace_path = temp_dir.path().to_path_buf();

    let programs_dir = workspace_path.join("programs");
    let project_dir = programs_dir.join("TestProgram");
    std::fs::create_dir_all(&project_dir).expect("Failed to create directories");

    // Create program file
    std::fs::write(
        programs_dir.join("TestProgram.md"),
        "---\ntitle: TestProgram\nstatus: New\n---\n",
    )
    .expect("Failed to create program file");

    // Create project file
    std::fs::create_dir_all(project_dir.join("projects").join("NewProject"))
        .expect("Failed to create project directories");
    std::fs::write(
        project_dir
            .join("projects")
            .join("NewProject")
            .join("NewProject.md"),
        "---\ntitle: NewProject\nstatus: New\n---\n",
    )
    .expect("Failed to create project file");

    // Set up config with temp workspace
    let config = crate::config::Config {
        workspace: workspace_path.clone(),
        ..crate::config::Config::default()
    };

    let mut app = App::new(config);

    // Navigate into program, then project
    app.navigation_state.selected_entry_index = 1;
    app.open_tree_item();
    app.navigation_state.selected_entry_index = 1;
    app.open_tree_item();

    // Start the new milestone wizard
    app.start_new_milestone();

    // Fill in the milestone name
    if let Some(ref mut state) = app.wizard_state.template {
        for field in &mut state.fields {
            if field.is_editable {
                field.value = "NewMilestone".to_string();
                break;
            }
        }
        state.focus = WizardFocus::ConfirmButton;
    }
    app.input_buffer = "NewMilestone".to_string();

    // Confirm the creation
    app.confirm_template_field();

    // Verify the file was created on disk
    let milestone_path = workspace_path
        .join("programs")
        .join("TestProgram")
        .join("projects")
        .join("NewProject")
        .join("milestones")
        .join("NewMilestone")
        .join("NewMilestone.md");
    assert!(
        milestone_path.exists(),
        "Milestone file should be created at {:?}",
        milestone_path
    );

    // Verify the file has content
    let content = std::fs::read_to_string(&milestone_path).expect("Should be able to read file");
    assert!(
        content.contains("NewMilestone"),
        "Milestone file should contain the name"
    );
}

#[test]
fn test_wizard_creates_task_file_on_disk() {
    // Create a temp workspace with program, project, and milestone
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let workspace_path = temp_dir.path().to_path_buf();

    let milestone_dir = workspace_path
        .join("programs")
        .join("TestProgram")
        .join("projects")
        .join("NewProject")
        .join("milestones")
        .join("NewMilestone");
    std::fs::create_dir_all(&milestone_dir).expect("Failed to create directories");

    // Create program file
    std::fs::write(
        workspace_path
            .join("programs")
            .join("TestProgram")
            .join("TestProgram.md"),
        "---\ntitle: TestProgram\nstatus: New\n---\n",
    )
    .expect("Failed to create program file");

    // Create project file
    std::fs::write(
        workspace_path
            .join("programs")
            .join("TestProgram")
            .join("projects")
            .join("NewProject")
            .join("NewProject.md"),
        "---\ntitle: NewProject\nstatus: New\n---\n",
    )
    .expect("Failed to create project file");

    // Create milestone file
    std::fs::write(
        milestone_dir.join("NewMilestone.md"),
        "---\ntitle: NewMilestone\nstatus: New\n---\n",
    )
    .expect("Failed to create milestone file");

    // Set up config with temp workspace
    let config = crate::config::Config {
        workspace: workspace_path.clone(),
        ..crate::config::Config::default()
    };

    let mut app = App::new(config);

    // Navigate into program, then project.
    // Right-navigation auto-selects first child, so after entering project
    // selection is already on the milestone.
    app.navigation_state.selected_entry_index = 1;
    app.open_tree_item();
    let project_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|i| i.name == "NewProject" && i.indent == 1)
        .expect("NewProject should be selectable");
    app.navigation_state.selected_entry_index = project_idx;
    app.open_tree_item();

    // Start the new task wizard
    app.start_new_task();

    // Fill in the task name
    if let Some(ref mut state) = app.wizard_state.template {
        for field in &mut state.fields {
            if field.is_editable {
                field.value = "NewTask".to_string();
                break;
            }
        }
        state.focus = WizardFocus::ConfirmButton;
    }
    app.input_buffer = "NewTask".to_string();

    // Confirm the creation
    app.confirm_template_field();

    // Verify the file was created on disk
    let task_path = workspace_path
        .join("programs")
        .join("TestProgram")
        .join("projects")
        .join("NewProject")
        .join("milestones")
        .join("NewMilestone")
        .join("tasks")
        .join("NewTask")
        .join("NewTask.md");
    assert!(
        task_path.exists(),
        "Task file should be created at {:?}",
        task_path
    );

    // Verify the file has content
    let content = std::fs::read_to_string(&task_path).expect("Should be able to read file");
    assert!(
        content.contains("NewTask"),
        "Task file should contain the name"
    );
}

#[test]
fn test_wizard_creates_subtask_file_on_disk() {
    // Create a temp workspace with program, project, milestone, and task
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let workspace_path = temp_dir.path().to_path_buf();

    let task_dir = workspace_path
        .join("programs")
        .join("TestProgram")
        .join("projects")
        .join("NewProject")
        .join("milestones")
        .join("NewMilestone")
        .join("tasks")
        .join("NewTask");
    std::fs::create_dir_all(&task_dir).expect("Failed to create directories");

    std::fs::write(
        workspace_path
            .join("programs")
            .join("TestProgram")
            .join("TestProgram.md"),
        "---\ntitle: TestProgram\nstatus: New\n---\n",
    )
    .expect("Failed to create program file");
    std::fs::write(
        workspace_path
            .join("programs")
            .join("TestProgram")
            .join("projects")
            .join("NewProject")
            .join("NewProject.md"),
        "---\ntitle: NewProject\nstatus: New\n---\n",
    )
    .expect("Failed to create project file");
    std::fs::write(
        workspace_path
            .join("programs")
            .join("TestProgram")
            .join("projects")
            .join("NewProject")
            .join("milestones")
            .join("NewMilestone")
            .join("NewMilestone.md"),
        "---\ntitle: NewMilestone\nstatus: New\n---\n",
    )
    .expect("Failed to create milestone file");
    std::fs::write(
        task_dir.join("NewTask.md"),
        "---\ntitle: NewTask\nstatus: New\n---\n",
    )
    .expect("Failed to create task file");

    let config = crate::config::Config {
        workspace: workspace_path.clone(),
        ..crate::config::Config::default()
    };
    let mut app = App::new(config);

    // Navigate into program, project, and milestone.
    app.navigation_state.selected_entry_index = 1;
    app.open_tree_item();
    let project_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|i| i.name == "NewProject" && i.indent == 1)
        .expect("NewProject should be selectable");
    app.navigation_state.selected_entry_index = project_idx;
    app.open_tree_item();

    // Open milestone so task appears and is selected by one-step right semantics.
    let milestone_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|i| i.name == "NewMilestone" && i.indent == 2)
        .expect("NewMilestone should be selectable");
    app.navigation_state.selected_entry_index = milestone_idx;
    app.open_tree_item();

    // Start the subtask wizard and fill in name.
    app.start_new_subtask();
    if let Some(ref mut state) = app.wizard_state.template {
        for field in &mut state.fields {
            if field.is_editable && field.placeholder.as_deref() == Some("NAME") {
                field.value = "NewSubtask".to_string();
                break;
            }
        }
        state.focus = WizardFocus::ConfirmButton;
    }
    app.input_buffer = "NewSubtask".to_string();
    app.confirm_template_field();

    let subtask_path = workspace_path
        .join("programs")
        .join("TestProgram")
        .join("projects")
        .join("NewProject")
        .join("milestones")
        .join("NewMilestone")
        .join("tasks")
        .join("NewTask")
        .join("subtasks")
        .join("NewSubtask")
        .join("NewSubtask.md");
    assert!(
        subtask_path.exists(),
        "Subtask file should be created at {:?}",
        subtask_path
    );

    let content =
        std::fs::read_to_string(&subtask_path).expect("Should be able to read subtask file");
    assert!(
        content.contains("type: subtask"),
        "Subtask file should use subtask template"
    );
}

#[test]
fn test_e_opens_metadata_editor_for_program_and_saves() {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let workspace_path = temp_dir.path().to_path_buf();

    let program_path = workspace_path
        .join("programs")
        .join("MyProgram")
        .join("MyProgram.md");
    if let Some(parent) = program_path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create directories");
    }
    let content = r#"---
uuid: prog-uuid
title: MyProgram
importance: low
status: New
creation_date: 2026-04-01
created_by: Test
type: program
---

# Description
Original
"#;
    std::fs::write(&program_path, content).expect("Failed to create program file");

    let config = crate::config::Config {
        workspace: workspace_path.clone(),
        ..crate::config::Config::default()
    };
    let mut app = App::new(config);

    // Select the program and press 'e'
    app.navigation_state.selected_entry_index = 1;
    app.handle_key(KeyCode::Char('e'));
    assert_eq!(app.current_view, ViewType::InputTemplateField);
    assert!(app.template_edit_target_path.is_some());

    if let Some(ref mut state) = app.wizard_state.template {
        for field in &mut state.fields {
            if field.placeholder.as_deref() == Some("DEFAULT_STATUS") {
                field.value = "Active".to_string();
            }
        }
        state.focus = WizardFocus::ConfirmButton;
    }

    app.confirm_template_field();
    assert_eq!(app.current_view, ViewType::TreeView);

    let updated = std::fs::read_to_string(&program_path).expect("Should read updated file");
    assert!(
        updated.contains("status: Active"),
        "Expected updated status in program file"
    );
}

#[test]
fn test_edit_subtask_metadata_in_tree_writes_file() {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let workspace_path = temp_dir.path().to_path_buf();

    let subtask_path = workspace_path
        .join("programs")
        .join("TestProgram")
        .join("projects")
        .join("NewProject")
        .join("milestones")
        .join("NewMilestone")
        .join("tasks")
        .join("NewTask")
        .join("subtasks")
        .join("NewSubtask")
        .join("NewSubtask.md");
    if let Some(parent) = subtask_path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create subtask directories");
    }

    std::fs::write(
        workspace_path
            .join("programs")
            .join("TestProgram")
            .join("TestProgram.md"),
        "---\ntitle: TestProgram\nstatus: New\n---\n",
    )
    .expect("Failed to create program file");
    std::fs::write(
        workspace_path
            .join("programs")
            .join("TestProgram")
            .join("projects")
            .join("NewProject")
            .join("NewProject.md"),
        "---\ntitle: NewProject\nstatus: New\n---\n",
    )
    .expect("Failed to create project file");
    std::fs::write(
        workspace_path
            .join("programs")
            .join("TestProgram")
            .join("projects")
            .join("NewProject")
            .join("milestones")
            .join("NewMilestone")
            .join("NewMilestone.md"),
        "---\ntitle: NewMilestone\nstatus: New\n---\n",
    )
    .expect("Failed to create milestone file");
    std::fs::write(
        workspace_path
            .join("programs")
            .join("TestProgram")
            .join("projects")
            .join("NewProject")
            .join("milestones")
            .join("NewMilestone")
            .join("tasks")
            .join("NewTask")
            .join("NewTask.md"),
        "---\ntitle: NewTask\nstatus: New\ntype: task\n---\n",
    )
    .expect("Failed to create task file");

    let subtask_content = r#"---
uuid: test-subtask-uuid
title: NewSubtask
status: New
type: subtask
---

# Description
Original
"#;
    std::fs::write(&subtask_path, subtask_content).expect("Failed to create subtask file");

    let config = crate::config::Config {
        workspace: workspace_path.clone(),
        ..crate::config::Config::default()
    };
    let mut app = App::new(config);

    app.set_selected_tree_path(vec![
        "TestProgram".to_string(),
        "NewProject".to_string(),
        "NewMilestone".to_string(),
        "NewTask".to_string(),
        "NewSubtask".to_string(),
    ]);
    app.load_tree_view_data();

    app.handle_key(KeyCode::Char('e'));
    assert_eq!(app.current_view, ViewType::InputTemplateField);
    if let Some(ref mut state) = app.wizard_state.template {
        for field in &mut state.fields {
            if field.placeholder.as_deref() == Some("DEFAULT_STATUS") {
                field.value = "Completed".to_string();
            } else if field.placeholder.as_deref() == Some("ASSIGNED_TO") {
                field.value = "Tay".to_string();
            } else if field.placeholder.as_deref() == Some("IMPORTANCE") {
                field.value = "high".to_string();
            }
        }
        state.focus = WizardFocus::ConfirmButton;
    }
    app.confirm_template_field();

    assert_eq!(app.current_view, ViewType::TreeView);

    let updated = std::fs::read_to_string(&subtask_path).expect("Should read updated subtask");
    let parsed = crate::storage::md::parse_element(&updated)
        .expect("Should parse updated subtask")
        .expect("Should parse updated subtask element");
    if let crate::model::Element::Task(task) = parsed {
        assert_eq!(task.status, "Completed");
        assert_eq!(task.assigned_to.as_deref(), Some("Tay"));
        assert_eq!(task.importance.as_deref(), Some("high"));
    } else {
        panic!("Updated subtask should parse as task-like element");
    }
}

#[test]
fn test_navigator_selection_stays_on_newly_created_element() {
    // Test that after creating an element, the navigator selects the new element
    // instead of resetting to the first item

    // Create a temp workspace
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let workspace_path = temp_dir.path().to_path_buf();

    // Create a program directory manually so we have something to work with
    let programs_dir = workspace_path.join("programs");
    std::fs::create_dir_all(&programs_dir).expect("Failed to create programs dir");

    // Create a simple program file
    let program_content = r#"---
uuid: test-uuid
title: TestProgram
status: New
tags: program
---

# DESCRIPTION
Test description
"#;
    std::fs::write(programs_dir.join("TestProgram.md"), program_content)
        .expect("Failed to create program file");

    // Set up config with temp workspace
    let config = crate::config::Config {
        workspace: workspace_path.clone(),
        ..crate::config::Config::default()
    };

    let mut app = App::new(config);

    // We're at root level - verify there are programs
    assert!(
        !app.tree_data.programs.is_empty(),
        "Programs should be loaded"
    );

    // Record the initial selection position (before creating new element)
    let _initial_selected_index = app.navigation_state.selected_entry_index;

    // Start the new program wizard
    app.start_new_program();

    // Fill in the program name in the first editable field
    if let Some(ref mut state) = app.wizard_state.template {
        for field in &mut state.fields {
            if field.is_editable && field.placeholder.as_deref() == Some("NAME") {
                field.value = "NewProgram".to_string();
                break;
            }
        }
        state.focus = WizardFocus::ConfirmButton;
    }
    app.input_buffer = "NewProgram".to_string();

    // Confirm the creation
    app.confirm_template_field();

    // Now verify selection is on the newly created element, NOT reset to first item
    // The new element should be in the sidebar
    let new_element_in_sidebar = app
        .navigation_state
        .sidebar_items
        .iter()
        .any(|item| item.name == "NewProgram");

    assert!(
        new_element_in_sidebar,
        "Newly created element should be in sidebar"
    );

    // The key assertion: selected_entry_index should point to the new element
    let selected_item =
        &app.navigation_state.sidebar_items[app.navigation_state.selected_entry_index];
    assert_eq!(
        selected_item.name, "NewProgram",
        "Selected item should be the newly created program, but got '{}' (index {})",
        selected_item.name, app.navigation_state.selected_entry_index
    );

    // Also verify we didn't just reset to initial position (index 1)
    // The new element should NOT be at index 1 if there's an existing program
    // (index 1 should be the first existing element "TestProgram", not "NewProgram")
}

#[test]
fn test_wizard_description_in_markdown_body() {
    // Test that DESCRIPTION field appears in wizard and is placed in markdown body

    // Create a temp workspace
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let workspace_path = temp_dir.path().to_path_buf();

    // Set up config with temp workspace
    let config = crate::config::Config {
        workspace: workspace_path.clone(),
        ..crate::config::Config::default()
    };

    let mut app = App::new(config);

    // Start the new program wizard
    app.start_new_program();

    // Verify we're in template field input mode
    assert_eq!(app.current_view, ViewType::InputTemplateField);
    assert!(app.wizard_state.template.is_some());

    // Check that DESCRIPTION is in the wizard fields
    let has_description_field = app
        .wizard_state
        .template
        .as_ref()
        .map(|state| {
            state
                .fields
                .iter()
                .any(|f| f.placeholder.as_deref() == Some("DESCRIPTION"))
        })
        .unwrap_or(false);

    assert!(
        has_description_field,
        "Wizard should have DESCRIPTION field"
    );

    // Fill in the program name and description
    if let Some(ref mut state) = app.wizard_state.template {
        for field in &mut state.fields {
            if field.placeholder.as_deref() == Some("NAME") {
                field.value = "TestProgram".to_string();
            } else if field.placeholder.as_deref() == Some("DESCRIPTION") {
                field.value = "This is a test description".to_string();
            }
        }
        state.focus = WizardFocus::ConfirmButton;
    }
    app.input_buffer = "TestProgram".to_string();

    // Confirm the creation
    app.confirm_template_field();

    // Verify the file was created on disk
    let program_path = workspace_path
        .join("programs")
        .join("TestProgram")
        .join("TestProgram.md");
    assert!(program_path.exists(), "Program file should be created");

    // Read the content and verify DESCRIPTION is in the markdown body
    let content = std::fs::read_to_string(&program_path).expect("Should be able to read file");

    // Verify description appears in markdown body (after YAML separator)
    assert!(
        content.contains("This is a test description"),
        "Description should appear in markdown body, got: {}",
        content
    );

    // Verify description is NOT in YAML frontmatter
    // The YAML section is between the two --- markers
    let yaml_section = content.split("---").nth(1).unwrap_or("");
    assert!(
        !yaml_section.to_lowercase().contains("description:"),
        "Description should NOT be in YAML frontmatter, YAML section was: {}",
        yaml_section
    );
}

#[test]
fn test_navigate_left_from_milestone() {
    // Create temp workspace
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let workspace_path = temp_dir.path().to_path_buf();

    // Create program: programs/TestProgram/TestProgram.md (nested structure)
    let program_dir = workspace_path.join("programs").join("TestProgram");
    std::fs::create_dir_all(&program_dir).expect("Failed to create program dir");
    std::fs::write(
        program_dir.join("TestProgram.md"),
        "---
title: TestProgram
---
# Test Program",
    )
    .expect("Failed to create program file");

    // Create project: programs/TestProgram/projects/TestProject/TestProject.md
    let project_dir = program_dir.join("projects").join("TestProject");
    std::fs::create_dir_all(&project_dir).expect("Failed to create project dir");
    std::fs::write(
        project_dir.join("TestProject.md"),
        "---
title: TestProject
---
# Test Project",
    )
    .expect("Failed to create project file");

    // Create milestone: programs/TestProgram/projects/TestProject/milestones/TestMilestone/TestMilestone.md
    let milestone_dir = project_dir.join("milestones").join("TestMilestone");
    std::fs::create_dir_all(&milestone_dir).expect("Failed to create milestone dir");
    std::fs::write(
        milestone_dir.join("TestMilestone.md"),
        "---
title: TestMilestone
---
# Test Milestone",
    )
    .expect("Failed to create milestone file");

    // Set up config with temp workspace
    let config = crate::config::Config {
        workspace: workspace_path.clone(),
        ..crate::config::Config::default()
    };

    let mut app = App::new(config);

    // Navigate into Program (select index 1 because index 0 is "Programs" header)
    app.navigation_state.selected_entry_index = 1;
    app.open_tree_item();

    // Navigate into Project
    let project_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|i| i.name == "TestProject")
        .expect("TestProject should be in sidebar");
    app.navigation_state.selected_entry_index = project_idx;
    app.open_tree_item();

    // Single right from project now expands and moves selection into milestone.
    assert_eq!(
        app.navigation_state.sidebar_tree.selected_depth(),
        3,
        "Should be inside milestone after second navigation"
    );

    // Now press left arrow to navigate back
    app.navigate_left();

    // BUG: This should go to project level (path = ["TestProgram", "TestProject"])
    // but it jumps to program level (path = ["TestProgram"])
    assert_eq!(
        app.navigation_state.sidebar_tree.selected_depth(),
        2,
        "Should go back to project level (depth 2), not program level (depth 1)"
    );
}

#[test]
fn test_navigate_left_shows_correct_sidebar() {
    // Create temp workspace
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let workspace_path = temp_dir.path().to_path_buf();

    // Create program: programs/TestProgram/TestProgram.md (nested structure)
    let program_dir = workspace_path.join("programs").join("TestProgram");
    std::fs::create_dir_all(&program_dir).expect("Failed to create program dir");
    std::fs::write(
        program_dir.join("TestProgram.md"),
        "---
title: TestProgram
---
# Test Program",
    )
    .expect("Failed to create program file");

    // Create project: programs/TestProgram/projects/TestProject/TestProject.md
    let project_dir = program_dir.join("projects").join("TestProject");
    std::fs::create_dir_all(&project_dir).expect("Failed to create project dir");
    std::fs::write(
        project_dir.join("TestProject.md"),
        "---
title: TestProject
---
# Test Project",
    )
    .expect("Failed to create project file");

    // Create milestone: programs/TestProgram/projects/TestProject/milestones/TestMilestone/TestMilestone.md
    let milestone_dir = project_dir.join("milestones").join("TestMilestone");
    std::fs::create_dir_all(&milestone_dir).expect("Failed to create milestone dir");
    std::fs::write(
        milestone_dir.join("TestMilestone.md"),
        "---
title: TestMilestone
---
# Test Milestone",
    )
    .expect("Failed to create milestone file");

    // Set up config with temp workspace
    let config = crate::config::Config {
        workspace: workspace_path.clone(),
        ..crate::config::Config::default()
    };

    let mut app = App::new(config);

    // Navigate into Program.
    // Right now expands and moves selection to the first project in one step.
    app.navigation_state.selected_entry_index = 1;
    app.open_tree_item();
    assert_eq!(app.navigation_state.sidebar_tree.selected_depth(), 2);

    // Navigate into Project
    let project_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|i| i.name == "TestProject")
        .expect("TestProject should be in sidebar");
    app.navigation_state.selected_entry_index = project_idx;
    app.open_tree_item();
    assert_eq!(app.navigation_state.sidebar_tree.selected_depth(), 3);

    // Now navigate LEFT - this should collapse back to project level
    app.navigate_left();

    // After collapsing, we should be at project level (depth 2)
    // path should be ["TestProgram", "TestProject"], not ["TestProgram"]
    assert_eq!(
        app.navigation_state.sidebar_tree.selected_depth(),
        2,
        "After collapsing milestone, should be at project level (depth 2), not program level (depth 1)"
    );

    // The sidebar should keep milestones collapsed while project is selected.
    let has_milestones = app
        .navigation_state
        .sidebar_items
        .iter()
        .any(|i| i.name == "TestMilestone");
    assert!(
        !has_milestones,
        "Sidebar should keep selected project's children collapsed"
    );

    // Selection should remain valid and on the project node after collapsing back.
    assert!(
        app.navigation_state.selected_entry_index < app.navigation_state.sidebar_items.len(),
        "Selected index should remain in bounds"
    );
    let selected = &app.navigation_state.sidebar_items[app.navigation_state.selected_entry_index];
    assert!(!selected.is_header, "Selection should not land on a header");
    assert_eq!(selected.name, "TestProject");
    assert_eq!(selected.indent, 1);
}

#[test]
fn test_entering_program_does_not_auto_expand_first_project_children() {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let workspace_path = temp_dir.path().to_path_buf();

    let program_dir = workspace_path.join("programs").join("TestProgram");
    std::fs::create_dir_all(&program_dir).expect("Failed to create program dir");
    std::fs::write(
        program_dir.join("TestProgram.md"),
        "---\ntitle: TestProgram\n---\n",
    )
    .expect("Failed to create program file");

    let alpha_project_dir = program_dir.join("projects").join("AlphaProject");
    std::fs::create_dir_all(&alpha_project_dir).expect("Failed to create alpha project dir");
    std::fs::write(
        alpha_project_dir.join("AlphaProject.md"),
        "---\ntitle: AlphaProject\n---\n",
    )
    .expect("Failed to create alpha project file");

    let beta_project_dir = program_dir.join("projects").join("BetaProject");
    std::fs::create_dir_all(&beta_project_dir).expect("Failed to create beta project dir");
    std::fs::write(
        beta_project_dir.join("BetaProject.md"),
        "---\ntitle: BetaProject\n---\n",
    )
    .expect("Failed to create beta project file");

    let milestone_dir = alpha_project_dir.join("milestones").join("M1");
    std::fs::create_dir_all(&milestone_dir).expect("Failed to create milestone dir");
    std::fs::write(milestone_dir.join("M1.md"), "---\ntitle: M1\n---\n")
        .expect("Failed to create milestone file");

    let config = crate::config::Config {
        workspace: workspace_path,
        ..crate::config::Config::default()
    };
    let mut app = App::new(config);

    app.navigation_state.selected_entry_index = 1;
    app.open_tree_item();

    assert_eq!(
        app.navigation_state.sidebar_tree.selected_path().to_vec(),
        vec!["TestProgram".to_string(), "AlphaProject".to_string()]
    );
    assert!(
        !app.navigation_state
            .sidebar_items
            .iter()
            .any(|item| item.name == "M1" && item.indent == 2),
        "Milestones should not auto-expand when entering program level"
    );
}

#[test]
fn test_navigate_left_collapses_children_of_newly_selected_parent() {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let workspace_path = temp_dir.path().to_path_buf();

    let program_dir = workspace_path.join("programs").join("TestProgram");
    std::fs::create_dir_all(&program_dir).expect("Failed to create program dir");
    std::fs::write(
        program_dir.join("TestProgram.md"),
        "---\ntitle: TestProgram\n---\n",
    )
    .expect("Failed to create program file");

    let project_dir = program_dir.join("projects").join("TestProject");
    std::fs::create_dir_all(&project_dir).expect("Failed to create project dir");
    std::fs::write(
        project_dir.join("TestProject.md"),
        "---\ntitle: TestProject\n---\n",
    )
    .expect("Failed to create project file");

    let milestone_dir = project_dir.join("milestones").join("M1");
    std::fs::create_dir_all(&milestone_dir).expect("Failed to create milestone dir");
    std::fs::write(milestone_dir.join("M1.md"), "---\ntitle: M1\n---\n")
        .expect("Failed to create milestone file");

    let config = crate::config::Config {
        workspace: workspace_path,
        ..crate::config::Config::default()
    };
    let mut app = App::new(config);

    app.navigation_state.selected_entry_index = 1;
    app.open_tree_item_with_leaf_open(false);
    let project_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|i| i.name == "TestProject" && i.indent == 1)
        .expect("TestProject should be selectable");
    app.navigation_state.selected_entry_index = project_idx;
    app.open_tree_item_with_leaf_open(false);

    assert_eq!(
        app.navigation_state.sidebar_tree.selected_path().to_vec(),
        vec![
            "TestProgram".to_string(),
            "TestProject".to_string(),
            "M1".to_string()
        ]
    );

    app.navigate_left();

    assert_eq!(
        app.navigation_state.sidebar_tree.selected_path().to_vec(),
        vec!["TestProgram".to_string(), "TestProject".to_string()]
    );
    assert!(
        !app.navigation_state
            .sidebar_items
            .iter()
            .any(|item| item.name == "M1" && item.indent == 2),
        "Milestones should be collapsed when project is selected after left navigation"
    );
}

#[test]
fn test_navigate_right_on_leaf_does_not_open_content_or_lose_selection() {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let workspace_path = temp_dir.path().to_path_buf();

    let task_dir = workspace_path
        .join("programs")
        .join("TestProgram")
        .join("projects")
        .join("TestProject")
        .join("milestones")
        .join("M1")
        .join("tasks")
        .join("LeafTask");
    std::fs::create_dir_all(&task_dir).expect("Failed to create task dir");
    std::fs::write(
        workspace_path
            .join("programs")
            .join("TestProgram")
            .join("TestProgram.md"),
        "---\ntitle: TestProgram\n---\n",
    )
    .expect("Failed to create program file");
    std::fs::write(
        workspace_path
            .join("programs")
            .join("TestProgram")
            .join("projects")
            .join("TestProject")
            .join("TestProject.md"),
        "---\ntitle: TestProject\n---\n",
    )
    .expect("Failed to create project file");
    std::fs::write(
        workspace_path
            .join("programs")
            .join("TestProgram")
            .join("projects")
            .join("TestProject")
            .join("milestones")
            .join("M1")
            .join("M1.md"),
        "---\ntitle: M1\n---\n",
    )
    .expect("Failed to create milestone file");
    std::fs::write(task_dir.join("LeafTask.md"), "---\ntitle: LeafTask\n---\n")
        .expect("Failed to create task file");

    let config = crate::config::Config {
        workspace: workspace_path,
        ..crate::config::Config::default()
    };
    let mut app = App::new(config);

    app.navigation_state.selected_entry_index = 1;
    app.open_tree_item_with_leaf_open(false);
    let project_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|i| i.name == "TestProject" && i.indent == 1)
        .expect("TestProject should be selectable");
    app.navigation_state.selected_entry_index = project_idx;
    app.open_tree_item_with_leaf_open(false);
    let milestone_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|i| i.name == "M1" && i.indent == 2)
        .expect("M1 should be selectable");
    app.navigation_state.selected_entry_index = milestone_idx;
    app.open_tree_item_with_leaf_open(false);
    let task_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|i| i.name == "LeafTask" && i.indent == 3)
        .expect("LeafTask should be selectable after entering milestone");
    app.navigation_state.selected_entry_index = task_idx;
    app.open_tree_item_with_leaf_open(false);

    let selected_before = app.navigation_state.sidebar_tree.selected_path().to_vec();
    app.navigate_right();
    app.navigate_right();

    assert_eq!(app.current_view, ViewType::TreeView);
    assert_eq!(
        app.navigation_state.sidebar_tree.selected_path().to_vec(),
        selected_before
    );
    assert!(app.navigation_state.selected_entry_index < app.navigation_state.sidebar_items.len());
}

#[test]
fn test_create_today_journal_from_template_populates_fields() {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let workspace_path = temp_dir.path().to_path_buf();

    let config = crate::config::Config {
        workspace: workspace_path.clone(),
        ..crate::config::Config::default()
    };
    let app = App::new(config);

    let today_path = app.config.workspace.today_journal_path();
    assert!(!today_path.exists());

    let created = app
        .create_today_journal_from_template()
        .expect("Journal creation should succeed");
    assert_eq!(created, today_path);
    assert!(today_path.exists());

    let content = std::fs::read_to_string(&today_path).expect("Should read created journal");
    assert!(
        !content.contains("{{UUID}}") && !content.contains("{{TODAY}}"),
        "Journal template placeholders should be resolved"
    );
}

#[test]
fn test_today_entry_does_not_open_wizard_when_present() {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let workspace_path = temp_dir.path().to_path_buf();

    let config = crate::config::Config {
        workspace: workspace_path.clone(),
        ..crate::config::Config::default()
    };
    let mut app = App::new(config);

    let today_path = app.config.workspace.today_journal_path();
    if let Some(parent) = today_path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create journal directory");
    }
    std::fs::write(&today_path, "---\ntitle: today\n---\n").expect("Failed to create file");

    let today_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|item| item.is_journal_item.as_deref() == Some("Today"))
        .expect("Today entry should exist");
    app.navigation_state.selected_entry_index = today_idx;
    app.open_tree_item();

    assert_eq!(app.current_view, ViewType::TreeView);
    assert!(app.wizard_state.template.is_none());
}

#[test]
fn test_left_navigation_preserves_project_selection_after_project_milestone_roundtrip() {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let workspace_path = temp_dir.path().to_path_buf();

    let program_dir = workspace_path.join("programs").join("TestProgram");
    std::fs::create_dir_all(&program_dir).expect("Failed to create program dir");
    std::fs::write(
        program_dir.join("TestProgram.md"),
        "---\ntitle: TestProgram\n---\n",
    )
    .expect("Failed to create program file");

    // Two projects to exercise up/down before navigating deeper.
    for project_name in ["AlphaProject", "BetaProject"] {
        let project_dir = program_dir.join("projects").join(project_name);
        std::fs::create_dir_all(&project_dir).expect("Failed to create project dir");
        std::fs::write(
            project_dir.join(format!("{project_name}.md")),
            format!("---\ntitle: {project_name}\n---\n"),
        )
        .expect("Failed to create project file");

        let milestone_dir = project_dir.join("milestones").join("M1");
        std::fs::create_dir_all(&milestone_dir).expect("Failed to create milestone dir");
        std::fs::write(milestone_dir.join("M1.md"), "---\ntitle: M1\n---\n")
            .expect("Failed to create milestone file");
    }

    let config = crate::config::Config {
        workspace: workspace_path,
        ..crate::config::Config::default()
    };
    let mut app = App::new(config);

    // Enter the top program.
    app.navigation_state.selected_entry_index = 1;
    app.open_tree_item();

    // Move selection in project list and enter BetaProject.
    if let Some(beta_idx) = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|i| i.name == "BetaProject" && i.indent == 1)
    {
        app.navigation_state.selected_entry_index = beta_idx;
    } else {
        panic!("BetaProject should be selectable");
    }
    app.open_tree_item();

    // Single right from project expands and moves to milestone.
    assert_eq!(
        app.navigation_state.sidebar_tree.selected_path().to_vec(),
        vec!["TestProgram", "BetaProject", "M1"]
    );

    // Collapse back one level.
    app.navigate_left();

    assert_eq!(
        app.navigation_state.sidebar_tree.selected_path().to_vec(),
        vec!["TestProgram", "BetaProject"]
    );
    assert!(app.navigation_state.selected_entry_index < app.navigation_state.sidebar_items.len());
    let selected = &app.navigation_state.sidebar_items[app.navigation_state.selected_entry_index];
    assert_eq!(selected.name, "BetaProject");
    assert_eq!(selected.indent, 1);
    assert!(!selected.is_header);
}

#[test]
fn test_tree_navigation_supports_program_with_direct_tasks() {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let workspace_path = temp_dir.path().to_path_buf();

    let program_dir = workspace_path.join("programs").join("TestProgram");
    let tasks_dir = program_dir.join("tasks");
    std::fs::create_dir_all(&tasks_dir).expect("Failed to create directories");
    std::fs::write(
        workspace_path.join("programs").join("TestProgram.md"),
        "---\ntitle: TestProgram\n---\n",
    )
    .expect("Failed to create program file");
    std::fs::write(
        tasks_dir.join("DirectTask.md"),
        "---\ntitle: DirectTask\n---\n",
    )
    .expect("Failed to create direct task file");

    let config = crate::config::Config {
        workspace: workspace_path,
        ..crate::config::Config::default()
    };
    let mut app = App::new(config);

    app.navigation_state.selected_entry_index = 1;
    app.open_tree_item();
    assert_eq!(
        app.navigation_state.sidebar_tree.selected_path().to_vec(),
        vec!["TestProgram", "DirectTask"]
    );

    app.navigate_left();
    assert_eq!(
        app.navigation_state.sidebar_tree.selected_path().to_vec(),
        vec!["TestProgram"]
    );
    assert!(app.navigation_state.selected_entry_index < app.navigation_state.sidebar_items.len());
}

#[test]
fn test_tree_navigation_prefers_expandable_variant_for_duplicate_names() {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let workspace_path = temp_dir.path().to_path_buf();

    let program_dir = workspace_path.join("programs").join("TestProgram");
    let project_container = program_dir.join("projects");
    std::fs::create_dir_all(&project_container).expect("Failed to create directories");

    std::fs::write(
        workspace_path.join("programs").join("TestProgram.md"),
        "---\ntitle: TestProgram\n---\n",
    )
    .expect("Failed to create program file");

    // Duplicate project name in flat and nested forms.
    std::fs::write(program_dir.join("Common.md"), "---\ntitle: Common\n---\n")
        .expect("Failed to create flat duplicate project file");
    let common_nested_dir = project_container.join("Common");
    std::fs::create_dir_all(common_nested_dir.join("milestones").join("M1"))
        .expect("Failed to create nested duplicate hierarchy");
    std::fs::write(
        common_nested_dir.join("Common.md"),
        "---\ntitle: Common\n---\n",
    )
    .expect("Failed to create nested duplicate project file");
    std::fs::write(
        common_nested_dir
            .join("milestones")
            .join("M1")
            .join("M1.md"),
        "---\ntitle: M1\n---\n",
    )
    .expect("Failed to create milestone file");

    let config = crate::config::Config {
        workspace: workspace_path,
        ..crate::config::Config::default()
    };
    let mut app = App::new(config);

    app.navigation_state.selected_entry_index = 1;
    app.open_tree_item();
    let common_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|i| i.name == "Common" && i.indent == 1)
        .expect("Common should be selectable under program");
    app.navigation_state.selected_entry_index = common_idx;
    app.open_tree_item();

    assert_eq!(
        app.navigation_state.sidebar_tree.selected_path().to_vec(),
        vec!["TestProgram", "Common", "M1"]
    );
    assert!(
        app.navigation_state
            .sidebar_items
            .iter()
            .any(|i| i.name == "M1" && i.indent == 2),
        "Expandable duplicate variant should be used, exposing milestones"
    );
}

#[test]
fn test_tree_navigation_does_not_flatten_non_container_dirs() {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let workspace_path = temp_dir.path().to_path_buf();

    let program_dir = workspace_path.join("programs").join("example program");
    let project_dir = program_dir.join("project 1");
    std::fs::create_dir_all(&project_dir).expect("Failed to create project dir");
    std::fs::write(
        workspace_path.join("programs").join("example program.md"),
        "---\ntitle: example program\n---\n",
    )
    .expect("Failed to create program file");
    std::fs::write(
        program_dir.join("project 1.md"),
        "---\ntitle: project 1\n---\n",
    )
    .expect("Failed to create project file");
    std::fs::write(
        project_dir.join("milestone 1.md"),
        "---\ntitle: milestone 1\n---\n",
    )
    .expect("Failed to create milestone file");
    std::fs::write(
        program_dir.join("project 2.md"),
        "---\ntitle: project 2\n---\n",
    )
    .expect("Failed to create second project file");

    let config = crate::config::Config {
        workspace: workspace_path,
        ..crate::config::Config::default()
    };
    let mut app = App::new(config);

    app.navigation_state.selected_entry_index = 1;
    app.open_tree_item();

    assert!(
        app.navigation_state
            .sidebar_items
            .iter()
            .any(|i| i.name == "project 1" && i.indent == 1),
        "project 1 should exist as a project under program"
    );
    assert!(
        app.navigation_state
            .sidebar_items
            .iter()
            .any(|i| i.name == "project 2" && i.indent == 1),
        "project 2 should exist as a project under program"
    );
    assert!(
        !app.navigation_state
            .sidebar_items
            .iter()
            .any(|i| i.name == "milestone 1" && i.indent == 1),
        "milestone 1 must not leak into program level"
    );

    let project_idx = app
        .navigation_state
        .sidebar_items
        .iter()
        .position(|i| i.name == "project 1" && i.indent == 1)
        .expect("project 1 should be selectable");
    app.navigation_state.selected_entry_index = project_idx;
    app.open_tree_item();

    assert!(
        app.navigation_state
            .sidebar_items
            .iter()
            .any(|i| i.name == "milestone 1" && i.indent == 2),
        "milestone 1 should appear only under project 1"
    );
}
