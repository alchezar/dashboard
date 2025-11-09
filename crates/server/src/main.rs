use dashboard_common::prelude::Result;
use dashboard_common::telemetry;
use dashboard_server::app::App;
use dashboard_server::config::Config;
use dashboard_server::model::queries;
use dashboard_server::proxmox::client::ProxmoxClient;
use dashboard_server::state::AppState;
use std::sync::Arc;
use tracing::Level;

/// The main entry point for the server application.
///
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging.
    let subscriber = telemetry::get_subscriber(Level::TRACE, std::io::stdout);
    telemetry::init_subscriber(subscriber)?;
    tracing::info!(target: "server", "Start!");
    tracing::info!(target: "server", "Logger ready.");

    let config = Config::from_env()?;
    let address = config.get_address();
    let app_state = AppState {
        pool: queries::connect_to_db(&config).await?,
        proxmox: Arc::new(ProxmoxClient::new(
            config.proxmox.url.clone(),
            config.proxmox.auth_header.clone(),
        )?),
        config,
    };
    let app = App::build(app_state, address).await?;
    tracing::info!(target: "server", "Listening on '{}'\n", app.get_url()?);

    app.run().await
}
