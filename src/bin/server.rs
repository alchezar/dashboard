use dashboard::app::App;
use dashboard::config::CONFIG;
use dashboard::prelude::{AppState, Result};
use dashboard::proxmox::client::ProxmoxClient;
use dashboard::telemetry;
use std::sync::Arc;
use tracing::Level;

/// The main entry point for the server application.
///
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging.
    let subscriber = telemetry::get_subscriber(Level::TRACE, std::io::stdout);
    telemetry::init_subscriber(subscriber)?;
    tracing::info!(target: ">> server", "Start!");
    tracing::info!(target: ">> server", "Logger ready.");

    let app_state = AppState {
        pool: dashboard::model::queries::connect_to_db().await?,
        proxmox: Arc::new(ProxmoxClient::new(
            CONFIG.proxmox_url.clone(),
            CONFIG.proxmox_auth_header.clone(),
        )),
    };
    let address = CONFIG.get_address()?;
    let app = App::build(app_state, address).await?;
    tracing::info!(target: ">> server", "Listening on '{}'\n", app.get_url()?);

    app.run().await
}
