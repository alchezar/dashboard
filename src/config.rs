use crate::prelude::Result;
use serde::Deserialize;
use std::sync::LazyLock;

pub static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    Config::from_env().unwrap_or_else(|error| panic!("FATAL - FAILED TO LOAD CONFIG: {:?}", error))
});

#[derive(Debug, Deserialize)]
pub struct Config {
    pub pwd_key: String,
    pub token_secret: String,
    pub token_duration_sec: u64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv()?;
        let cfg = envy::from_env::<Config>()?;
        Ok(cfg)
    }
}
