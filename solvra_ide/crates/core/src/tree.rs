use crate::error::SolvraIdeError;
use ignore::{DirEntry, WalkBuilder};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectNode {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub children: Vec<ProjectNode>,
}

impl ProjectNode {
    pub fn new(name: String, path: PathBuf, is_dir: bool) -> Self {
        Self {
            name,
            path,
            is_dir,
            children: Vec::new(),
        }
    }

    pub fn with_children(mut self, children: Vec<ProjectNode>) -> Self {
        self.children = children;
        self
    }
}

#[derive(Default)]
pub struct ProjectTreeBuilder {
    pub ignore_hidden: bool,
}

impl ProjectTreeBuilder {
    pub fn build(&self, root: &Path) -> Result<ProjectNode, SolvraIdeError> {
        let mut builder = WalkBuilder::new(root);
        builder.git_ignore(true).hidden(self.ignore_hidden);
        let mut entries: Vec<DirEntry> = builder
            .build()
            .filter_map(Result::ok)
            .filter(|entry| entry.path() != root)
            .collect();
        entries.sort_by_key(|entry| entry.path().to_path_buf());

        let mut root_node = ProjectNode::new(
            root.file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| root.display().to_string()),
            root.to_path_buf(),
            true,
        );

        for entry in entries {
            let path = entry.path().to_path_buf();
            let relative = path.strip_prefix(root).unwrap().to_path_buf();
            Self::insert(&mut root_node, relative, path.is_dir());
        }

        Ok(root_node)
    }

    fn insert(node: &mut ProjectNode, relative: PathBuf, is_dir: bool) {
        if let Some((first, rest)) = split_path(relative) {
            let first_name = first.to_string_lossy().to_string();
            let parent_path = node.path.clone();
            if rest.as_os_str().is_empty() {
                node.children.push(ProjectNode::new(
                    first_name,
                    parent_path.join(&first),
                    is_dir,
                ));
            } else {
                let new_child_name = first_name.clone();
                let child = node
                    .children
                    .iter_mut()
                    .find(|child| child.name == first_name);
                if let Some(child) = child {
                    Self::insert(child, rest, is_dir);
                } else {
                    let mut new_child =
                        ProjectNode::new(new_child_name, parent_path.join(&first), true);
                    Self::insert(&mut new_child, rest, is_dir);
                    node.children.push(new_child);
                }
            }
        }
    }
}

fn split_path(path: PathBuf) -> Option<(PathBuf, PathBuf)> {
    let mut components = path.components();
    let first = components.next()?;
    let remainder: PathBuf = components.collect();
    Some((PathBuf::from(first.as_os_str()), remainder))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn builds_tree() {
        let tmp = tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("src/main.svs"), "let x = 1").unwrap();
        let builder = ProjectTreeBuilder::default();
        let tree = builder.build(tmp.path()).unwrap();
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].name, "src");
    }
}
