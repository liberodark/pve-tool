use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct ProxmoxClient {
    base_url: String,
    token: Option<String>,
    client: reqwest::Client,
}

impl ProxmoxClient {
    pub fn new(host: &str, port: u16, token: Option<String>, verify_ssl: bool) -> Result<Self> {
        let base_url = format!("https://{}:{}/api2/json", host, port);

        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(!verify_ssl)
            .build()?;

        Ok(Self {
            base_url,
            token,
            client,
        })
    }

    pub async fn get<T: for<'de> Deserialize<'de>>(&self, endpoint: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, endpoint);
        let mut request = self.client.get(&url);

        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("PVEAPIToken={}", token));
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            anyhow::bail!("API request failed with status {}: {}", status, text);
        }

        let data: ApiResponse<T> = response.json().await?;
        Ok(data.data)
    }

    pub async fn post<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
        data: &T,
    ) -> Result<R> {
        let url = format!("{}{}", self.base_url, endpoint);
        let mut request = self.client.post(&url);

        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("PVEAPIToken={}", token));
        }

        let response = request.form(data).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            anyhow::bail!("API request failed with status {}: {}", status, text);
        }

        let result: ApiResponse<R> = response.json().await?;
        Ok(result.data)
    }

    pub async fn delete(&self, endpoint: &str) -> Result<String> {
        let url = format!("{}{}", self.base_url, endpoint);
        let mut request = self.client.delete(&url);

        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("PVEAPIToken={}", token));
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            anyhow::bail!("API request failed with status {}: {}", status, text);
        }

        let result: ApiResponse<String> = response.json().await?;
        Ok(result.data)
    }
}

#[derive(Deserialize)]
struct ApiResponse<T> {
    data: T,
}
