//=============================================
// nova_compositor/src/wm.rs
//=============================================
// Author: Nova GUI Team
// License: MIT
// Goal: Skeleton window/workspace manager
// Objective: Provide data structures for tiling and focus bookkeeping
//=============================================

/// Representation of a workspace slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    /// Logical name (e.g., "1", "dev", "chat").
    pub name: String,
    /// Whether the workspace currently has focus.
    pub focused: bool,
}

impl Workspace {
    /// Create a new workspace with the provided name.
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            focused: false,
        }
    }
}

/// Container for managing workspaces and focus order.
#[derive(Debug)]
pub struct WorkspaceManager {
    workspaces: Vec<Workspace>,
    focus_index: usize,
}

impl WorkspaceManager {
    /// Build a manager with the default workspace list.
    pub fn new() -> Self {
        let mut mgr = Self {
            workspaces: vec![
                Workspace::named("1"),
                Workspace::named("2"),
                Workspace::named("3"),
            ],
            focus_index: 0,
        };
        if let Some(first) = mgr.workspaces.first_mut() {
            first.focused = true;
        }
        mgr
    }

    /// Number of workspaces tracked by the manager.
    pub fn len(&self) -> usize {
        self.workspaces.len()
    }

    /// Advance focus to the next workspace.
    pub fn focus_next(&mut self) {
        if self.workspaces.is_empty() {
            return;
        }
        self.workspaces[self.focus_index].focused = false;
        self.focus_index = (self.focus_index + 1) % self.workspaces.len();
        self.workspaces[self.focus_index].focused = true;
    }
}
