//! Sidebar tree model for unified navigation.
//!
//! Provides a depth-agnostic tree model for sidebar navigation that replaces
//! both the Programs TreeModel and JournalTreeState with a single abstraction.

use std::collections::BTreeSet;

#[derive(Debug, Clone, Default)]
pub struct SidebarTreeModel<N> {
    selected_path: Vec<String>,
    expanded_paths: BTreeSet<Vec<String>>,
    #[allow(dead_code)]
    _phantom: std::marker::PhantomData<N>,
}

impl<N> SidebarTreeModel<N> {
    #[must_use]
    pub fn selected_path(&self) -> &[String] {
        &self.selected_path
    }

    #[must_use]
    pub fn selected_path_vec(&self) -> Vec<String> {
        self.selected_path.clone()
    }

    #[must_use]
    pub fn selected_depth(&self) -> usize {
        self.selected_path.len()
    }

    pub fn set_selected_path(&mut self, path: Vec<String>) {
        self.selected_path = path;
    }

    pub fn expand_ancestors(&mut self, path: &[String]) {
        for depth in 1..path.len() {
            self.expanded_paths.insert(path[..depth].to_vec());
        }
    }

    pub fn expand_path(&mut self, path: &[String]) {
        self.expanded_paths.insert(path.to_vec());
    }

    pub fn collapse_path(&mut self, path: &[String]) {
        self.expanded_paths
            .retain(|expanded| !is_same_or_descendant(expanded, path));
    }

    #[must_use]
    pub fn is_expanded(&self, path: &[String]) -> bool {
        self.expanded_paths.contains(path)
    }

    #[must_use]
    pub fn parent_path(&self) -> Vec<String> {
        let mut p = self.selected_path.clone();
        p.pop();
        p
    }

    pub fn has_expanded_descendants(&self, path: &[String]) -> bool {
        self.expanded_paths.iter().any(|e| is_child_of(e, path))
    }

    pub fn expanded_paths_count(&self) -> usize {
        self.expanded_paths.len()
    }

    #[must_use]
    pub fn expanded_paths(&self) -> &BTreeSet<Vec<String>> {
        &self.expanded_paths
    }

    pub fn clear_expanded(&mut self) {
        self.expanded_paths.clear();
    }

    pub fn reset(&mut self) {
        self.selected_path.clear();
        self.expanded_paths.clear();
    }
}

fn is_same_or_descendant(candidate: &[String], ancestor: &[String]) -> bool {
    candidate.len() >= ancestor.len() && candidate.starts_with(ancestor)
}

fn is_child_of(candidate: &[String], parent: &[String]) -> bool {
    candidate.len() == parent.len() + 1 && candidate.starts_with(parent)
}

#[cfg(test)]
mod tests {
    use super::SidebarTreeModel;

    #[test]
    fn test_default_collapsed() {
        let model: SidebarTreeModel<()> = SidebarTreeModel::default();
        assert!(model.selected_path().is_empty());
        assert_eq!(model.selected_depth(), 0);
        assert_eq!(model.expanded_paths_count(), 0);
    }

    #[test]
    fn test_set_selected_expands_ancestors() {
        let mut model: SidebarTreeModel<()> = SidebarTreeModel::default();
        model.set_selected_path(vec!["Alpha".to_string(), "Beta".to_string()]);
        model.expand_ancestors(&["Alpha".to_string(), "Beta".to_string()]);

        assert_eq!(
            model.selected_path(),
            &["Alpha".to_string(), "Beta".to_string()]
        );
        assert!(model.is_expanded(&["Alpha".to_string()]));
        assert!(!model.is_expanded(&["Alpha".to_string(), "Beta".to_string()]));
    }

    #[test]
    fn test_expand_single_child() {
        let mut model: SidebarTreeModel<()> = SidebarTreeModel::default();
        model.set_selected_path(vec!["Alpha".to_string()]);

        model.expand_path(&["Alpha".to_string()]);
        assert!(model.is_expanded(&["Alpha".to_string()]));

        model.set_selected_path(vec!["Alpha".to_string(), "Beta".to_string()]);
        assert!(model.is_expanded(&["Alpha".to_string()]));
    }

    #[test]
    fn test_collapse_path_removes_descendants() {
        let mut model: SidebarTreeModel<()> = SidebarTreeModel::default();
        model.set_selected_path(vec![
            "Alpha".to_string(),
            "Beta".to_string(),
            "Gamma".to_string(),
        ]);
        model.expand_ancestors(&["Alpha".to_string(), "Beta".to_string(), "Gamma".to_string()]);

        model.collapse_path(&["Alpha".to_string(), "Beta".to_string()]);

        assert!(!model.is_expanded(&["Alpha".to_string(), "Beta".to_string()]));
        assert!(!model.is_expanded(&[
            "Alpha".to_string(),
            "Beta".to_string(),
            "Gamma".to_string(),
        ]));
        assert!(model.is_expanded(&["Alpha".to_string()]));
    }

    #[test]
    fn test_expand_level_2() {
        let mut model: SidebarTreeModel<()> = SidebarTreeModel::default();
        model.set_selected_path(vec!["Alpha".to_string()]);
        model.expand_path(&["Alpha".to_string()]);
        model.expand_path(&["Alpha".to_string(), "Beta1".to_string()]);

        assert!(model.is_expanded(&["Alpha".to_string()]));
        assert!(model.is_expanded(&["Alpha".to_string(), "Beta1".to_string()]));
        assert!(!model.is_expanded(&["Alpha".to_string(), "Beta2".to_string()]));
    }

    #[test]
    fn test_expand_level_3() {
        let mut model: SidebarTreeModel<()> = SidebarTreeModel::default();
        model.set_selected_path(vec!["Alpha".to_string()]);
        model.expand_path(&["Alpha".to_string()]);
        model.expand_path(&["Alpha".to_string(), "Beta1".to_string()]);
        model.expand_path(&[
            "Alpha".to_string(),
            "Beta1".to_string(),
            "Gamma1".to_string(),
        ]);
        model.expand_path(&[
            "Alpha".to_string(),
            "Beta1".to_string(),
            "Gamma2".to_string(),
        ]);
        model.expand_path(&[
            "Alpha".to_string(),
            "Beta1".to_string(),
            "Gamma3".to_string(),
        ]);

        assert!(model.is_expanded(&["Alpha".to_string()]));
        assert!(model.is_expanded(&["Alpha".to_string(), "Beta1".to_string()]));
        assert!(model.is_expanded(&[
            "Alpha".to_string(),
            "Beta1".to_string(),
            "Gamma1".to_string()
        ]));
        assert!(model.is_expanded(&[
            "Alpha".to_string(),
            "Beta1".to_string(),
            "Gamma2".to_string()
        ]));
        assert!(model.is_expanded(&[
            "Alpha".to_string(),
            "Beta1".to_string(),
            "Gamma3".to_string()
        ]));
    }

    #[test]
    fn test_collapse_moves_selection_to_parent() {
        let mut model: SidebarTreeModel<()> = SidebarTreeModel::default();
        model.set_selected_path(vec![
            "Alpha".to_string(),
            "Beta1".to_string(),
            "Gamma2".to_string(),
            "Delta1".to_string(),
        ]);
        model.expand_ancestors(&[
            "Alpha".to_string(),
            "Beta1".to_string(),
            "Gamma2".to_string(),
            "Delta1".to_string(),
        ]);

        model.collapse_path(&[
            "Alpha".to_string(),
            "Beta1".to_string(),
            "Gamma2".to_string(),
        ]);
        model.set_selected_path(model.parent_path());

        assert_eq!(
            model.selected_path(),
            &[
                "Alpha".to_string(),
                "Beta1".to_string(),
                "Gamma2".to_string()
            ]
        );
        assert!(!model.is_expanded(&[
            "Alpha".to_string(),
            "Beta1".to_string(),
            "Gamma2".to_string()
        ]));
        assert!(model.is_expanded(&["Alpha".to_string()]));
        assert!(model.is_expanded(&["Alpha".to_string(), "Beta1".to_string()]));
    }

    #[test]
    fn test_expand_preserves_sibling_subtrees() {
        let mut model: SidebarTreeModel<()> = SidebarTreeModel::default();
        model.expand_path(&["Alpha".to_string()]);
        model.expand_path(&["Alpha".to_string(), "Beta1".to_string()]);
        model.expand_path(&[
            "Alpha".to_string(),
            "Beta1".to_string(),
            "Gamma1".to_string(),
        ]);
        model.expand_path(&[
            "Alpha".to_string(),
            "Beta1".to_string(),
            "Gamma2".to_string(),
        ]);
        model.expand_path(&[
            "Alpha".to_string(),
            "Beta1".to_string(),
            "Gamma2".to_string(),
            "Delta1".to_string(),
        ]);
        model.expand_path(&[
            "Alpha".to_string(),
            "Beta1".to_string(),
            "Gamma2".to_string(),
            "Delta2".to_string(),
        ]);
        model.expand_path(&[
            "Alpha".to_string(),
            "Beta1".to_string(),
            "Gamma2".to_string(),
            "Delta3".to_string(),
        ]);
        model.expand_path(&[
            "Alpha".to_string(),
            "Beta1".to_string(),
            "Gamma3".to_string(),
        ]);

        model.expand_path(&[
            "Alpha".to_string(),
            "Beta1".to_string(),
            "Gamma3".to_string(),
        ]);
        model.expand_path(&[
            "Alpha".to_string(),
            "Beta1".to_string(),
            "Gamma3".to_string(),
            "Delta4".to_string(),
        ]);
        model.expand_path(&[
            "Alpha".to_string(),
            "Beta1".to_string(),
            "Gamma3".to_string(),
            "Delta5".to_string(),
        ]);
        model.expand_path(&[
            "Alpha".to_string(),
            "Beta1".to_string(),
            "Gamma3".to_string(),
            "Delta6".to_string(),
        ]);

        assert!(model.is_expanded(&[
            "Alpha".to_string(),
            "Beta1".to_string(),
            "Gamma2".to_string(),
            "Delta1".to_string()
        ]));
        assert!(model.is_expanded(&[
            "Alpha".to_string(),
            "Beta1".to_string(),
            "Gamma2".to_string(),
            "Delta2".to_string()
        ]));
        assert!(model.is_expanded(&[
            "Alpha".to_string(),
            "Beta1".to_string(),
            "Gamma2".to_string(),
            "Delta3".to_string()
        ]));
        assert!(model.is_expanded(&[
            "Alpha".to_string(),
            "Beta1".to_string(),
            "Gamma3".to_string(),
            "Delta4".to_string()
        ]));
        assert!(model.is_expanded(&[
            "Alpha".to_string(),
            "Beta1".to_string(),
            "Gamma3".to_string(),
            "Delta5".to_string()
        ]));
        assert!(model.is_expanded(&[
            "Alpha".to_string(),
            "Beta1".to_string(),
            "Gamma3".to_string(),
            "Delta6".to_string()
        ]));
    }

    #[test]
    fn test_parent_path() {
        let mut model: SidebarTreeModel<()> = SidebarTreeModel::default();
        model.set_selected_path(vec![
            "Alpha".to_string(),
            "Beta".to_string(),
            "Gamma".to_string(),
        ]);

        let parent = model.parent_path();
        assert_eq!(parent, vec!["Alpha".to_string(), "Beta".to_string()]);
    }

    #[test]
    fn test_parent_path_at_root() {
        let mut model: SidebarTreeModel<()> = SidebarTreeModel::default();
        model.set_selected_path(vec!["Alpha".to_string()]);

        let parent = model.parent_path();
        assert!(parent.is_empty());
    }

    #[test]
    fn test_has_expanded_descendants() {
        let mut model: SidebarTreeModel<()> = SidebarTreeModel::default();
        model.expand_path(&["Alpha".to_string()]);
        model.expand_path(&["Alpha".to_string(), "Beta".to_string()]);

        assert!(model.has_expanded_descendants(&["Alpha".to_string()]));
        assert!(!model.has_expanded_descendants(&["Alpha".to_string(), "Beta".to_string()]));
        assert!(!model.has_expanded_descendants(&["Alpha2".to_string()]));
    }

    #[test]
    fn test_clear_expanded() {
        let mut model: SidebarTreeModel<()> = SidebarTreeModel::default();
        model.expand_path(&["Alpha".to_string()]);
        model.expand_path(&["Alpha".to_string(), "Beta".to_string()]);

        model.clear_expanded();

        assert!(!model.is_expanded(&["Alpha".to_string()]));
        assert!(!model.is_expanded(&["Alpha".to_string(), "Beta".to_string()]));
        assert_eq!(model.selected_depth(), 0);
    }

    #[test]
    fn test_reset() {
        let mut model: SidebarTreeModel<()> = SidebarTreeModel::default();
        model.set_selected_path(vec!["Alpha".to_string(), "Beta".to_string()]);
        model.expand_path(&["Alpha".to_string()]);

        model.reset();

        assert!(model.selected_path().is_empty());
        assert_eq!(model.expanded_paths_count(), 0);
    }
}
