use std::collections::BTreeSet;

#[derive(Debug, Clone, Default)]
pub struct TreeModel {
    selected_path: Vec<String>,
    expanded_paths: BTreeSet<Vec<String>>,
}

impl TreeModel {
    #[must_use]
    pub fn selected_path(&self) -> &[String] {
        &self.selected_path
    }

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
}

fn is_same_or_descendant(candidate: &[String], ancestor: &[String]) -> bool {
    candidate.len() >= ancestor.len() && candidate.starts_with(ancestor)
}

#[cfg(test)]
mod tests {
    use super::TreeModel;

    #[test]
    fn test_set_selected_expands_prefixes() {
        let mut model = TreeModel::default();
        model.set_selected_path(vec!["Program".to_string(), "Project".to_string()]);
        model.expand_ancestors(&["Program".to_string(), "Project".to_string()]);

        assert_eq!(
            model.selected_path(),
            &["Program".to_string(), "Project".to_string()]
        );
        assert!(model.is_expanded(&["Program".to_string()]));
        assert!(!model.is_expanded(&["Program".to_string(), "Project".to_string()]));
    }

    #[test]
    fn test_collapse_path_removes_descendants() {
        let mut model = TreeModel::default();
        model.set_selected_path(vec![
            "Program".to_string(),
            "Project".to_string(),
            "Milestone".to_string(),
        ]);
        model.expand_ancestors(&[
            "Program".to_string(),
            "Project".to_string(),
            "Milestone".to_string(),
        ]);

        model.collapse_path(&["Program".to_string(), "Project".to_string()]);

        assert!(!model.is_expanded(&["Program".to_string(), "Project".to_string()]));
        assert!(!model.is_expanded(&[
            "Program".to_string(),
            "Project".to_string(),
            "Milestone".to_string(),
        ]));
        assert!(model.is_expanded(&["Program".to_string()]));
    }
}
