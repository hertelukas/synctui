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
    pub fn load<T>(path_arg: Option<T>) -> eyre::Result<Self>
    where
        T: Into<PathBuf>,
    {
        let effective_path: PathBuf = match path_arg {
            Some(p) => p.into(),
            None => {
                let base_config_dir =
                    dirs::config_dir().ok_or(AppError::DefaultConfigDirNotFound)?;
                base_config_dir.join("synctui").join("config.toml")
            }
        };

        let config_content =
            read_to_string(&effective_path).map_err(|io_error| AppError::ConfigReadError {
                path: effective_path.clone(),
                source: io_error,
            })?;

        let config_struct: Self =
            toml::from_str(&config_content).map_err(|toml_error| AppError::ConfigParseError {
                path: effective_path,
                source: toml_error,
            })?;

        Ok(config_struct)
    }
}
