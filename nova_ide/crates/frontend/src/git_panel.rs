use std::path::PathBuf;

use anyhow::{Context, Result};
use git2::{BranchType, Repository, Signature, Status, StatusOptions};

#[derive(Debug, Clone)]
pub struct FileChangeEntry {
    pub path: PathBuf,
    pub status: Status,
}

#[derive(Debug, Clone, Default)]
pub struct GitSummary {
    pub branch: Option<String>,
    pub changes: Vec<FileChangeEntry>,
}

pub struct GitPanelState {
    repo: Option<Repository>,
    pub root: PathBuf,
    pub summary: GitSummary,
}

impl GitPanelState {
    pub fn new(root: PathBuf) -> Self {
        let repo = Repository::discover(&root).ok();
        let summary = GitSummary::default();
        Self {
            repo,
            root,
            summary,
        }
    }

    pub fn refresh(&mut self) {
        if let Some(repo) = &self.repo {
            let mut options = StatusOptions::new();
            options.include_ignored(false).include_untracked(true);
            match repo.statuses(Some(&mut options)) {
                Ok(statuses) => {
                    let mut changes = Vec::new();
                    for entry in statuses.iter() {
                        if let Some(path) = entry.path() {
                            changes.push(FileChangeEntry {
                                path: self.root.join(path),
                                status: entry.status(),
                            });
                        }
                    }
                    self.summary.changes = changes;
                }
                Err(err) => {
                    self.summary.changes.clear();
                    log::warn!("Unable to read git status: {}", err);
                }
            }
            self.summary.branch = repo
                .head()
                .ok()
                .and_then(|head| head.shorthand().map(str::to_string));
        } else {
            self.summary.branch = None;
            self.summary.changes.clear();
        }
    }

    pub fn branch_names(&self) -> Vec<String> {
        if let Some(repo) = &self.repo {
            repo.branches(Some(BranchType::Local))
                .map(|iter| {
                    iter.filter_map(|branch| branch.ok())
                        .filter_map(|(branch, _)| branch.name().ok().flatten().map(str::to_string))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    pub fn checkout_branch(&mut self, name: &str) -> Result<()> {
        let repo = self
            .repo
            .as_ref()
            .context("Cannot checkout branch without a repository")?;
        let (object, reference) = repo.revparse_ext(name)?;
        repo.checkout_tree(&object, None)?;
        if let Some(reference) = reference {
            repo.set_head(reference.name().context("Missing head name")?)?;
        } else {
            repo.set_head_detached(object.id())?;
        }
        Ok(())
    }

    pub fn commit_all(&mut self, message: &str, signature: &Signature<'_>) -> Result<()> {
        let repo = self
            .repo
            .as_ref()
            .context("Cannot create commit without a repository")?;
        let mut index = repo.index()?;
        index.add_all(["*"], git2::IndexAddOption::DEFAULT, None)?;
        let tree_oid = index.write_tree()?;
        let tree = repo.find_tree(tree_oid)?;
        let head = repo.head().ok();
        let parents = if let Some(head) = head.and_then(|head| head.target()) {
            vec![repo.find_commit(head)?]
        } else {
            Vec::new()
        };
        let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
        repo.commit(
            Some("HEAD"),
            signature,
            signature,
            message,
            &tree,
            &parent_refs,
        )?;
        Ok(())
    }
}
