use crate::executor::{CommandOutcome, ExecutionContext};
use crate::registry::{CommandHandler, CommandInvocation};
use anyhow::{anyhow, Result};
use nova_app_store::{AppId, AppStore};
use std::str::FromStr;

/// Built-in command providing access to the Nova App Store from the CLI.
#[derive(Debug, Default)]
pub struct AppStoreCommand;

impl CommandHandler for AppStoreCommand {
    fn name(&self) -> &str {
        "app"
    }

    fn summary(&self) -> &str {
        "Interact with the Nova App Store (catalog, install, uninstall)"
    }

    fn handle(
        &self,
        _ctx: &mut ExecutionContext<'_>,
        invocation: &CommandInvocation,
    ) -> Result<CommandOutcome> {
        if invocation.args.is_empty() {
            return Ok(CommandOutcome::with_stdout(0, usage()));
        }
        let mut store = AppStore::bootstrap().map_err(|err| anyhow!(err))?;
        match invocation.args[0].as_str() {
            "catalog" => Ok(CommandOutcome::with_stdout(0, render_catalog(&store))),
            "installed" => Ok(CommandOutcome::with_stdout(0, render_installed(&store))),
            "info" => {
                let id = invocation
                    .args
                    .get(1)
                    .ok_or_else(|| anyhow!("app info requires an app identifier"))?;
                let app_id = AppId::from_str(id)?;
                render_info(&store, &app_id)
            }
            "install" => {
                let (app_id, version) = parse_install_args(&invocation.args[1..])?;
                let manifest = store.install(&app_id, version.as_ref().map(|v| v.as_ref()))?;
                let mut message =
                    format!("Installed {} version {}\n", manifest.id, manifest.version);
                if !manifest.capabilities.is_empty() {
                    message.push_str("Capabilities registered:\n");
                    for capability in &manifest.capabilities {
                        message.push_str(&format!("  - {}\n", capability.id));
                    }
                }
                Ok(CommandOutcome::with_stdout(0, message))
            }
            "uninstall" => {
                let id = invocation
                    .args
                    .get(1)
                    .ok_or_else(|| anyhow!("app uninstall requires an app identifier"))?;
                let app_id = AppId::from_str(id)?;
                store.uninstall(&app_id)?;
                Ok(CommandOutcome::with_stdout(
                    0,
                    format!("Removed application {}\n", app_id),
                ))
            }
            other => Err(anyhow!(
                "unknown app store subcommand '{}'. {}",
                other,
                usage()
            )),
        }
    }
}

fn render_catalog(store: &AppStore) -> String {
    let mut output = String::new();
    for (id, metadata) in store.list_available() {
        let latest = metadata
            .latest_package()
            .map(|pkg| pkg.version.to_string())
            .unwrap_or_else(|| "n/a".into());
        output.push_str(&format!(
            "{} ({})\n  {}\n  latest: {}\n\n",
            id, metadata.name, metadata.summary, latest
        ));
    }
    if output.is_empty() {
        output.push_str("Catalog is empty.\n");
    }
    output
}

fn render_installed(store: &AppStore) -> String {
    let mut output = String::new();
    for (id, manifest) in store.list_installed() {
        output.push_str(&format!(
            "{} {} (installed {})\n",
            id, manifest.version, manifest.installed_at
        ));
    }
    if output.is_empty() {
        output.push_str("No applications installed.\n");
    }
    output
}

fn render_info(store: &AppStore, id: &AppId) -> Result<CommandOutcome> {
    let metadata = store
        .metadata(id)
        .ok_or_else(|| anyhow!("application {} not found in catalog", id))?;
    let mut output = String::new();
    output.push_str(&format!("{}\n{}\n\n", metadata.name, metadata.summary));
    output.push_str("Tags: ");
    if metadata.tags.is_empty() {
        output.push_str("(none)\n");
    } else {
        output.push_str(&format!(
            "{}\n",
            metadata.tags.iter().cloned().collect::<Vec<_>>().join(", ")
        ));
    }
    if let Some(package) = metadata.latest_package() {
        output.push_str(&format!("Latest version: {}\n", package.version));
        if !package.capabilities.is_empty() {
            output.push_str("Capabilities:\n");
            for capability in &package.capabilities {
                output.push_str(&format!("  - {}\n", capability.description));
            }
        }
    }
    Ok(CommandOutcome::with_stdout(0, output))
}

fn parse_install_args(args: &[String]) -> Result<(AppId, Option<VersionWrapper>)> {
    if args.is_empty() {
        return Err(anyhow!("install requires an app identifier"));
    }
    let mut iter = args.iter();
    let id = iter.next().expect("checked non-empty slice").to_string();
    let mut version = None;
    while let Some(arg) = iter.next() {
        if arg == "--version" {
            let value = iter
                .next()
                .ok_or_else(|| anyhow!("--version flag requires a semantic version"))?;
            version = Some(VersionWrapper::parse(value)?);
        }
    }
    Ok((AppId::from_str(&id)?, version))
}

fn usage() -> String {
    "Usage: app <catalog|installed|info|install|uninstall> [options]\n".to_string()
}

/// Wrapper enabling optional semantic version argument parsing.
#[derive(Debug, Clone)]
struct VersionWrapper(semver::Version);

impl VersionWrapper {
    fn parse(input: &str) -> Result<Self> {
        let version = semver::Version::parse(input)
            .map_err(|_| anyhow!("invalid semantic version '{}'", input))?;
        Ok(Self(version))
    }
}

impl AsRef<semver::Version> for VersionWrapper {
    fn as_ref(&self) -> &semver::Version {
        &self.0
    }
}
