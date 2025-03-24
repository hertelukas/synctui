use color_eyre::eyre;
use reqwest::header;
use tokio::sync::mpsc::Sender;

use crate::{AppError, Configuration, Event, ty::Folder};

const ADDR: &str = "http://localhost:8384/rest";

/// Abstraction to interact with the syncthing API.
#[derive(Debug, Clone)]
pub struct Client {
    client: reqwest::Client,
}

impl Client {
    /// Creates a new HTTP client, with which the syncthing API can be used.
    /// The API can either be generated in the GUI of syncthing or set
    /// in the configuration file under `configuration/gui/apikey`.
    pub fn new(api_key: &str) -> eyre::Result<Self> {
        let mut headers = header::HeaderMap::new();
        let mut api_key_header = header::HeaderValue::from_str(api_key)?;
        api_key_header.set_sensitive(true);
        headers.insert("X-API-KEY", api_key_header);

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;
        Ok(Self { client })
    }

    /// Returns the syncthing ID of the local device
    pub async fn get_id(&self) -> eyre::Result<String, AppError> {
        Ok(self
            .client
            .get(format!("{}/system/ping", ADDR))
            .send()
            .await?
            .error_for_status()?
            .headers()
            .get("X-Syncthing-Id")
            .ok_or(AppError::SyncthingIDError)?
            .to_str()?
            .to_string())
    }

    pub async fn ping(&self) -> eyre::Result<()> {
        self.client
            .get(format!("{}/system/ping", ADDR))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    /// GET the entire config
    pub async fn get_configuration(&self) -> eyre::Result<Configuration, AppError> {
        Ok(self
            .client
            .get(format!("{}/config", ADDR))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    /// Only returns if an error is encountered.
    /// Transmits every new event over `tx`.
    pub async fn get_events(&self, tx: Sender<Event>) -> eyre::Result<()> {
        let mut current_id = 0;
        loop {
            let events: Vec<Event> = self
                .client
                .get(format!("{}/events?since={}", ADDR, current_id))
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;

            for event in events {
                current_id = event.id;
                tx.send(event).await?;
            }
        }
    }

    /// Creates a new folder, or updates it, if it already exists.
    pub async fn post_folder(&self, folder: Folder) -> eyre::Result<(), AppError> {
        self.client
            .post(format!("{}/config/folders", ADDR))
            .json(&folder)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}
