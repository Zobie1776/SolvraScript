use crate::error::NovaIdeError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

// ===========================================================================
// Workspace configuration
// ===========================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceConfig {
    #[serde(default)]
    pub task: TaskSection,
    #[serde(default)]
    pub project: Option<ProjectSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskSection {
    #[serde(default)]
    pub default: WorkspaceTaskSet,
}

impl Default for TaskSection {
    fn default() -> Self {
        Self {
            default: WorkspaceTaskSet::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectSection {
    pub name: Option<String>,
    #[serde(default)]
    pub autodetect: bool,
}

fn default_build() -> String {
    "nova build".into()
}

fn default_run() -> String {
    "nova run".into()
}

fn default_test() -> String {
    "cargo test".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceTaskSet {
    #[serde(default = "default_build")]
    pub build: String,
    #[serde(default = "default_run")]
    pub run: String,
    #[serde(default = "default_test")]
    pub test: String,
}

impl Default for WorkspaceTaskSet {
    fn default() -> Self {
        Self {
            build: default_build(),
            run: default_run(),
            test: default_test(),
        }
    }
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            task: TaskSection::default(),
            project: Some(ProjectSection {
                name: Some("NovaIDE".into()),
                autodetect: true,
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorkspaceLoader {
    path: PathBuf,
}

impl WorkspaceLoader {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn load(&self) -> Result<WorkspaceConfig, NovaIdeError> {
        if !self.path.exists() {
            return Ok(WorkspaceConfig::default());
        }
        let content = fs::read_to_string(&self.path)?;
        let mut config: WorkspaceConfig = toml::from_str(&content)
            .map_err(|err| NovaIdeError::workspace(format!("invalid workspace config: {err}")))?;

        if let Some(project) = &mut config.project {
            if project.name.as_deref().unwrap_or_default().is_empty() {
                project.name = Some("NovaIDE".into());
            }
        }

        Ok(config)
    }

    pub fn save(&self, config: &WorkspaceConfig) -> Result<(), NovaIdeError> {
        let content = toml::to_string_pretty(config)
            .map_err(|err| NovaIdeError::workspace(format!("failed to serialize config: {err}")))?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&self.path, content)?;
        Ok(())
    }

    pub fn infer_from_path(&self, project_root: &Path) -> Result<WorkspaceConfig, NovaIdeError> {
        if project_root.join("nova.toml").exists() {
            let name = project_root
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let config = WorkspaceConfig {
                project: Some(ProjectSection {
                    name: Some(name),
                    autodetect: true,
                }),
                ..WorkspaceConfig::default()
            };
            return Ok(config);
        }
        self.load()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn saves_and_loads() {
        let tmp = tempdir().unwrap();
        let loader = WorkspaceLoader::new(tmp.path().join("workspace.toml"));
        let config = WorkspaceConfig::default();
        loader.save(&config).unwrap();
        let roundtrip = loader.load().unwrap();
        assert_eq!(roundtrip.task.default.build, "nova build");
    }

    #[test]
    fn infer_uses_directory_name() {
        let tmp = tempdir().unwrap();
        std::fs::write(tmp.path().join("nova.toml"), "[package]\nname='demo'\n").unwrap();
        let loader = WorkspaceLoader::new(tmp.path().join("workspace.toml"));
        let config = loader.infer_from_path(tmp.path()).unwrap();
        assert!(config.project.unwrap().autodetect);
    }
}
