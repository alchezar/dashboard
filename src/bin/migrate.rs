use clap::Parser;
use dashboard::cli::Cli;
use dashboard::prelude::Result;
use dashboard::{model::migration, telemetry};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::postgres::PgPoolOptions;
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

    let source_pool = MySqlPoolOptions::new().connect(&cli.source_url).await?;
    let target_pool = PgPoolOptions::new().connect(&cli.target_url).await?;
    tracing::info!(?cli, "Database pools created.");

    migration::migrate(source_pool, target_pool, cli.dry_run).await?;
    tracing::info!(dry_run = %cli.dry_run, "Migration completed.");

    Ok(())
}
