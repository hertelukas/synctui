mod api;
pub use api::client::Client;

mod config;
pub use config::AppConfig;

mod error;
pub use error::AppError;

mod tui;
pub use tui::start;
