#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("no config found")]
    NoConfig,
}
