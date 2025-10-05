use dashboard::app::App;
use dashboard::config::CONFIG;
use dashboard::prelude::{AppState, Result};
use dashboard::proxmox::client::ProxmoxClient;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::Level;
use tracing_subscriber::fmt::format::FmtSpan;

/// The main entry point for the server application.
///
#[tokio::main]
async fn main() -> Result<()> {
    println!("Server!");
    init_logger()?;
    tracing::info!(target: ">> server", "Logger ready.");

    let app_state = AppState {
        pool: dashboard::model::queries::connect_to_db().await?,
        proxmox: Arc::new(ProxmoxClient::new(
            CONFIG.proxmox_url.clone(),
            CONFIG.proxmox_auth_header.clone(),
        )),
    };
    let address = SocketAddr::from(([127, 0, 0, 1], 8080));
    let app = App::build(app_state, address).await?;
    tracing::info!(target: ">> server", "Listening on '{}'\n", app.get_url()?);

    app.run().await
}

/// Initializes the global `tracing` subscriber for logging.
///
fn init_logger() -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(true)
        .with_max_level(Level::TRACE)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}
