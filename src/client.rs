use color_eyre::eyre;
use reqwest::header;

use crate::Configuration;

const ADDR: &str = "http://localhost:8384/rest";

#[derive(Debug)]
pub struct Client {
    client: reqwest::Client,
}

impl Client {
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

    pub async fn ping(&self) -> eyre::Result<()> {
        self.client
            .get(format!("{}/system/ping", ADDR))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn get_config(&self) -> eyre::Result<Configuration> {
        Ok(self
            .client
            .get(format!("{}/config", ADDR))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }
}
