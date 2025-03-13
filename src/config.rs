use std::{fs::read_to_string, path::PathBuf};

use color_eyre::eyre;
use serde::{Deserialize, Serialize};

use crate::AppError;

#[derive(Debug, Deserialize, Serialize)]
pub struct AppConfig {
    #[serde(rename = "api-key")]
    pub api_key: String,
}

impl AppConfig {
    pub fn load<T>(path: Option<T>) -> eyre::Result<Self>
    where
        T: Into<PathBuf>,
    {
        let path: PathBuf = match path {
            Some(path) => path.into(),
            None => dirs::config_dir()
                .ok_or(AppError::NoConfig)?
                .join("synctui")
                .join("config.toml"),
        };

        let config = read_to_string(path)?;

        Ok(toml::from_str(&config)?)
    }
}
