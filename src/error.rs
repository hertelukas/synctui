use std::path::PathBuf;

use crate::tui::state::Reload;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(
        "Could not determine the system's default configuration directory. This is needed to locate 'synctui/config.toml'."
    )]
    DefaultConfigDirNotFound,

    #[error("Failed to read configuration file from '{path}'")]
    ConfigReadError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to parse TOML configuration from '{path}'")]
    ConfigParseError {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error(transparent)]
    APIError(#[from] reqwest::Error),
    #[error("syncthing ID header not set")]
    SyncthingIDError,
    #[error(transparent)]
    ToStrError(#[from] reqwest::header::ToStrError),
    #[error("folder ID already exists")]
    DuplicateFolderID,
    #[error(transparent)]
    SendUnitError(#[from] tokio::sync::mpsc::error::SendError<()>),
    #[error(transparent)]
    SendReloadError(#[from] tokio::sync::mpsc::error::SendError<Reload>),
    #[error("folder not found")]
    UnknownFolder,
    #[error("device not found")]
    UnknownDevice,
    #[error("syncthing API error")]
    SyncthingError(#[from] syncthing_rs::error::Error),
}
