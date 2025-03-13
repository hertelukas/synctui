use std::fs::read_to_string;

use color_eyre::eyre;
use serde::{Deserialize, Serialize};

use crate::AppError;

#[derive(Debug, Deserialize, Serialize)]
pub struct AppConfig {
    #[serde(rename = "api-key")]
    pub api_key: String,
}

impl AppConfig {
    pub fn load() -> eyre::Result<Self> {
        let path = dirs::config_dir()
            .ok_or(AppError::NoConfig)?
            .join("synctui")
            .join("config.toml");

        let config = read_to_string(path)?;

        Ok(toml::from_str(&config)?)
    }
}
