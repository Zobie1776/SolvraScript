//! Rustyline helper providing SolvraCLI completions and hints.

use crate::registry::Registry;
use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::{Hint, Hinter};
use rustyline::validate::{ValidationContext, ValidationResult, Validator};
use rustyline::{Context as ReadlineContext, Helper};
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;
use walkdir::WalkDir;

/// Static hint implementation returning a single suggestion string.
#[derive(Clone, Debug)]
pub struct StaticHint(String);

impl Hint for StaticHint {
    fn display(&self) -> &str {
        &self.0
    }

    fn completion(&self) -> Option<&str> {
        Some(&self.0)
    }
}

/// Rustyline helper wiring completions from the registry and `$PATH`.
#[derive(Clone)]
pub struct SolvraHelper {
    registry: Arc<Registry>,
    executables: Arc<BTreeSet<String>>,
}

impl SolvraHelper {
    /// Create a new helper instance.
    pub fn new(registry: Arc<Registry>) -> Self {
        let mut executables = BTreeSet::new();
        if let Ok(path) = std::env::var("PATH") {
            for entry in path.split(':') {
                let dir = Path::new(entry);
                if dir.is_dir() {
                    for file in WalkDir::new(dir)
                        .max_depth(1)
                        .into_iter()
                        .filter_map(Result::ok)
                    {
                        let path = file.path();
                        if path.is_file() {
                            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                                executables.insert(name.to_string());
                            }
                        }
                    }
                }
            }
        }
        Self {
            registry,
            executables: Arc::new(executables),
        }
    }

    fn command_candidates(&self) -> Vec<String> {
        let mut names: BTreeSet<String> = self.registry.all_commands().into_iter().collect();
        names.extend(self.executables.iter().cloned());
        names.into_iter().collect()
    }
}

impl Helper for SolvraHelper {}

impl Completer for SolvraHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &ReadlineContext<'_>,
    ) -> Result<(usize, Vec<Pair>), rustyline::error::ReadlineError> {
        let start = line[..pos]
            .rfind(|c: char| [' ', '\t'].contains(&c))
            .map(|idx| idx + 1)
            .unwrap_or(0);
        let fragment = &line[start..pos];
        let candidates = self
            .command_candidates()
            .into_iter()
            .filter(|name| name.starts_with(fragment))
            .map(|name| Pair {
                display: name.clone(),
                replacement: name,
            })
            .collect();
        Ok((start, candidates))
    }
}

impl Hinter for SolvraHelper {
    type Hint = StaticHint;

    fn hint(&self, line: &str, pos: usize, _ctx: &ReadlineContext<'_>) -> Option<Self::Hint> {
        let (start, mut candidates) = self.complete(line, pos, _ctx).ok()?;
        if candidates.len() == 1 {
            let completion = candidates.remove(0).replacement;
            if completion.len() > pos - start {
                let suffix = &completion[pos - start..];
                return Some(StaticHint(suffix.to_string()));
            }
        }
        None
    }
}

impl Highlighter for SolvraHelper {}

impl Validator for SolvraHelper {
    fn validate(
        &self,
        ctx: &mut ValidationContext<'_>,
    ) -> Result<ValidationResult, rustyline::error::ReadlineError> {
        let input = ctx.input();
        if input.chars().filter(|c| *c == '{').count()
            == input.chars().filter(|c| *c == '}').count()
        {
            Ok(ValidationResult::Valid(None))
        } else {
            Ok(ValidationResult::Incomplete)
        }
    }
}
