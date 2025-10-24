use clap::Parser;
use common::error::Result;
use common::telemetry;
use migration_utility::cli::Cli;
use migration_utility::migration::Migration;
use tracing::Level;

/// The main entry point for the migration utility.
///
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging.
    let subscriber = telemetry::get_subscriber(Level::TRACE, std::io::stdout);
    telemetry::init_subscriber(subscriber)?;
    tracing::info!("Utility Start!");
    tracing::info!("Logger ready.");

    dotenv::dotenv()?;
    tracing::info!(".env loaded.");

    let cli = Cli::parse();
    tracing::info!(?cli, "Cli arguments parsed.");

    Migration::new(&cli).await?.run().await
}
