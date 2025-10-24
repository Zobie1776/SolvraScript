use std::path::{Path, PathBuf};

use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct ExplorerNode {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub expanded: bool,
    pub children: Vec<ExplorerNode>,
}

impl ExplorerNode {
    pub fn new(path: PathBuf, is_dir: bool) -> Self {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        Self {
            name,
            path,
            is_dir,
            expanded: false,
            children: Vec::new(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct FileExplorer {
    pub root: Option<ExplorerNode>,
}

impl FileExplorer {
    pub fn load_from_root(&mut self, root: PathBuf, max_depth: usize) {
        let mut node = ExplorerNode::new(root.clone(), true);
        node.expanded = true;
        node.children = self.build_tree(root.clone(), max_depth);
        self.root = Some(node);
    }

    pub fn refresh(&mut self, max_depth: usize) {
        if let Some(root) = self.root.as_ref().map(|node| node.path.clone()) {
            self.load_from_root(root, max_depth);
        }
    }

    fn build_tree(&self, root: PathBuf, max_depth: usize) -> Vec<ExplorerNode> {
        Self::collect_children(root.as_path(), max_depth)
    }

    fn collect_children(path: &Path, max_depth: usize) -> Vec<ExplorerNode> {
        let mut nodes = Vec::new();
        if max_depth == 0 {
            return nodes;
        }

        for entry in WalkDir::new(path)
            .max_depth(1)
            .into_iter()
            .filter_map(Result::ok)
        {
            let entry_path = entry.path();
            if entry_path == path {
                continue;
            }
            let is_dir = entry.file_type().is_dir();
            let mut node = ExplorerNode::new(entry_path.to_path_buf(), is_dir);
            if is_dir {
                node.children = Self::collect_children(entry_path, max_depth - 1);
            }
            nodes.push(node);
        }
        nodes.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        nodes
    }

    pub fn flatten_paths(&self) -> Vec<PathBuf> {
        fn collect(node: &ExplorerNode, buffer: &mut Vec<PathBuf>) {
            if !node.is_dir {
                buffer.push(node.path.clone());
            }
            for child in &node.children {
                collect(child, buffer);
            }
        }

        let mut paths = Vec::new();
        if let Some(root) = &self.root {
            for child in &root.children {
                collect(child, &mut paths);
            }
        }
        paths
    }
}
