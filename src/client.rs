use color_eyre::eyre;
use log::debug;
use reqwest::header;
use tokio::sync::mpsc::Sender;

use crate::{
    AppError, Configuration, Event,
    ty::{Device, Folder, PendingDevices, PendingFolders},
};

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
    #[must_use]
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
    #[must_use]
    pub async fn get_id(&self) -> eyre::Result<String, AppError> {
        debug!("GET /ping for ID");
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

    #[must_use]
    pub async fn ping(&self) -> eyre::Result<()> {
        debug!("GET /ping");
        self.client
            .get(format!("{}/system/ping", ADDR))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    /// GET the entire config
    #[must_use]
    pub async fn get_configuration(&self) -> eyre::Result<Configuration, AppError> {
        debug!("GET /config");
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
    /// If `skip_old` is set, all events before the call to this function do not
    /// result in a transmission.
    #[must_use]
    pub async fn get_events(
        &self,
        tx: Sender<Event>,
        mut skip_old: bool,
    ) -> eyre::Result<(), AppError> {
        let mut current_id = 0;
        loop {
            debug!("GET /events");
            let events: Vec<Event> = self
                .client
                .get(format!("{}/events?since={}", ADDR, current_id))
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;

            debug!("Received {} new events", events.len());
            for event in events {
                current_id = event.id;
                if !skip_old {
                    tx.send(event).await?;
                }
            }
            skip_old = false;
        }
    }

    /// Creates a new folder, or updates it, if it already exists.
    #[must_use]
    pub async fn post_folder(&self, folder: Folder) -> eyre::Result<(), AppError> {
        debug!("POST /config/folders {:#?}", folder);
        self.client
            .post(format!("{}/config/folders", ADDR))
            .json(&folder)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    /// Get a list of all pending remote devices which have tried to connect but
    /// aren't configured yet.
    #[must_use]
    pub async fn get_pending_devices(&self) -> eyre::Result<PendingDevices, AppError> {
        debug!("GET /cluster/pending/devices");
        Ok(self
            .client
            .get(format!("{}/cluster/pending/devices", ADDR))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    /// Folders which remote devices have offered to us, but are not yet shared from our
    /// instance to them.
    #[must_use]
    pub async fn get_pending_folders(&self) -> eyre::Result<PendingFolders, AppError> {
        debug!("GET /cluster/pending/folders");
        Ok(self
            .client
            .get(format!("{}/cluster/pending/folders", ADDR))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    /// Remove record about pending remote device with ID `device_id` which tried to connect.
    /// This is not permanent, use [`ignore_device`] instead.
    #[must_use]
    pub async fn delete_pending_device(&self, device_id: &str) -> eyre::Result<(), AppError> {
        debug!("DELETE /cluster/pending/devices?device={device_id}");
        self.client
            .delete(format!(
                "{}/cluster/pending/devices?device={}",
                ADDR, device_id
            ))
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    #[must_use]
    pub async fn add_device(&self, device: Device) -> eyre::Result<(), AppError> {
        debug!("POST /rest/config/devices");
        debug!("{:?}", device);
        self.client
            .post(format!("{}/config/devices", ADDR))
            .json(&device)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}
