mod api;
mod app_store;
mod ide;
mod ui;

use std::ops::ControlFlow;
use std::time::Duration;

use anyhow::Result;
use tokio::signal;
use tokio::time;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    install_tracing();

    let config = ui::UiConfig::default();
    let (mut ui_manager, mut events) = ui::LiteUiManager::bootstrap(config).await?;
    let mut ide = ide::MobileIde::default();
    let mut app_store = app_store::MobileAppStore::new();
    let mut ai_client = api::SolvraAiClient::new(api::ClientOptions::default()).await?;
    let event_tx = ui_manager.event_sender();

    let mut frame_interval = time::interval(Duration::from_millis(16));

    loop {
        tokio::select! {
            _ = frame_interval.tick() => {
                ui_manager
                    .render_frame(&mut ide, &mut app_store, &mut ai_client)
                    .await?;
            }
            maybe_event = events.recv() => {
                let Some(event) = maybe_event else { break; };
                if matches!(
                    ui_manager
                        .handle_event(event, &mut ide, &mut app_store, &mut ai_client)
                        .await?,
                    ControlFlow::Break(())
                ) {
                    break;
                }
            }
            _ = signal::ctrl_c() => {
                let _ = event_tx.send(ui::UiEvent::Shutdown);
            }
        }
    }

    ui_manager.shutdown().await;
    Ok(())
}

fn install_tracing() {
    let mut filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    if let Ok(directive) = "solvra_lite=info".parse() {
        filter = filter.add_directive(directive);
    }
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}
