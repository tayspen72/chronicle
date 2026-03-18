# NavigationState Refactor Plan

## Current Status

**NavigationState struct exists** in `src/tui/navigation.rs` with all 7 fields defined:
- `tree_model: TreeModel`
- `selected_entry_index: usize`
- `sidebar_items: Vec<SidebarItem>`
- `current_program: Option<String>`
- `current_project: Option<String>`
- `current_milestone: Option<String>`
- `current_task: Option<String>`

**App struct updated** with `navigation_state: NavigationState` field (line ~136).

**Remaining work**: 86 references to old fields in `mod.rs` need migration.

## Migration Strategy

### Approach: Replace references in batches

Instead of field-by-field, replace all occurrences of each old field pattern with the new struct accessor.

### Fields to Migrate (in order of complexity)

1. **`sidebar_items`** (~30 refs) - Used heavily in navigation methods
2. **`tree_model`** (~25 refs) - Core tree state
3. **`selected_entry_index`** (~15 refs) - Simple index
4. **`current_*`** (~16 refs) - Derived from tree_model

## Step-by-Step Execution

### Step 1: Replace sidebar_items references
```bash
# In mod.rs, replace:
self.sidebar_items → self.navigation_state.sidebar_items
app.sidebar_items → app.navigation_state.sidebar_items
```

### Step 2: Replace tree_model references
```bash
# In mod.rs, replace:
self.tree_model → self.navigation_state.tree_model
app.tree_model → app.navigation_state.tree_model
```

### Step 3: Replace selected_entry_index
```bash
# In mod.rs, replace:
self.selected_entry_index → self.navigation_state.selected_entry_index
app.selected_entry_index → app.navigation_state.selected_entry_index
```

### Step 4: Replace current_* fields
```bash
# In mod.rs, replace:
self.current_program → self.navigation_state.current_program
self.current_project → self.navigation_state.current_project
self.current_milestone → self.navigation_state.current_milestone
self.current_task → self.navigation_state.current_task
```

### Step 5: Update views/mod.rs
Similar replacements for rendering code that accesses navigation state.

### Step 6: Update layout.rs
Breadcrumb rendering uses current_* fields.

### Step 7: Remove old fields from App
Once all references updated, remove the duplicate fields from App struct:
- `tree_model: TreeModel`
- `selected_entry_index: usize`
- `sidebar_items: Vec<SidebarItem>`
- `current_program: Option<String>`
- `current_project: Option<String>`
- `current_milestone: Option<String>`
- `current_task: Option<String>`

## Risk Mitigation

1. **Keep navigation_state field** - Don't remove until all old fields fully migrated
2. **Test after each batch** - Run `cargo test` after each field group
3. **Check views/mod.rs** - Has ~10 refs to navigation fields
4. **Check layout.rs** - Has ~4 refs for breadcrumbs

## Verification Commands

```bash
# After each batch
cargo check
cargo test

# Final verification
cargo clippy
cargo test
```

## Estimated Time

- Step 1-4 (mod.rs): ~20 min
- Step 5-6 (views/layout): ~10 min
- Step 7 (cleanup): ~5 min
- **Total**: ~35-45 min

## Notes

- NavigationState already has helper methods `selected_path()` and `selected_depth()`
- Consider adding more helpers as migration progresses
- The `navigation::navigate_up()` and `navigation::navigate_down()` functions already take parameters, making them compatible
