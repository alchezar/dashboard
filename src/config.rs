use crate::prelude::Result;
use serde::Deserialize;
use std::env;
use std::sync::LazyLock;

pub static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    Config::from_env().unwrap_or_else(|error| panic!("== FAILED TO LOAD CONFIG: {:?}", error))
});

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct Config {
    pub database_url: String,
    pub password_secret: String,
    pub token_secret: String,
    pub token_duration_sec: u64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv()?;
        tracing::info!("-- .env loaded.");

        let config = Self {
            database_url: env::var("DATABASE_URL")?,
            password_secret: env::var("PASSWORD_SECRET")?,
            token_secret: env::var("TOKEN_SECRET")?,
            token_duration_sec: env::var("TOKEN_DURATION_SEC")?.parse::<u64>()?,
        };
        tracing::info!("-- Configuration loaded.");

        Ok(config)

        // let builder = config::Config::builder()
        //     .add_source(config::Environment::default())
        //     .build()?;
        //
        // let config = builder.try_deserialize()?;
        // tracing::info!("-- Config loaded: {:?}", config);
        //
        // Ok(config)
    }
}
