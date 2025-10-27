use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, clap::Parser)]
#[command(
    name = "Migration Utility",
    version = "0.1.0",
    about = "Command-line tool for WHMCS-to-PostgreSQL data transfer"
)]
pub struct Cli {
    #[arg(short, long, help("Perform the dry run without changing the database"))]
    pub dry_run: bool,
    #[arg(
        short,
        long,
        help = "Sets the number of records to process per batch",
        env = "CHUNK_SIZE"
    )]
    pub chunk_size: usize,
    #[arg(
        short,
        long,
        help = "Source database URL (WHMCS MySQL)",
        env = "SOURCE_URL"
    )]
    pub source_url: String,
    #[arg(
        short,
        long,
        help = "Target database URL (PostgreSQL)",
        env = "TARGET_URL"
    )]
    pub target_url: String,
}
