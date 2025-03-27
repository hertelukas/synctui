use crate::{tui::Reload, ty};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("no config found")]
    NoConfig,
    #[error(transparent)]
    APIError(#[from] reqwest::Error),
    #[error("syncthing ID header not set")]
    SyncthingIDError,
    #[error(transparent)]
    ToStrError(#[from] reqwest::header::ToStrError),
    #[error("folder ID already exists")]
    DuplicateFolderID,
    #[error(transparent)]
    SendEventError(#[from] tokio::sync::mpsc::error::SendError<ty::Event>),
    #[error(transparent)]
    SendUnitError(#[from] tokio::sync::mpsc::error::SendError<()>),
    #[error(transparent)]
    SendReloadError(#[from] tokio::sync::mpsc::error::SendError<Reload>),
}
