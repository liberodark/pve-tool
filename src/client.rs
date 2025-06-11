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

    fn parse_host_port(host: &str, default_port: u16) -> (String, u16) {
        if let Some((h, p)) = host.split_once(':') {
            if let Ok(port) = p.parse::<u16>() {
                (h.to_string(), port)
            } else {
                (host.to_string(), default_port)
            }
        } else {
            (host.to_string(), default_port)
        }
    }

    pub async fn new_with_fallback(
        hosts: &[String],
        default_port: u16,
        token: Option<String>,
        verify_ssl: bool,
    ) -> Result<Self> {
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(!verify_ssl)
            .build()?;

        for host_str in hosts {
            let (host, port) = Self::parse_host_port(host_str, default_port);
            let base_url = format!("https://{}:{}/api2/json", host, port);
            let test_client = Self {
                base_url: base_url.clone(),
                token: token.clone(),
                client: client.clone(),
            };

            if test_client
                .get::<serde_json::Value>("/version")
                .await
                .is_ok()
            {
                return Ok(test_client);
            }
        }

        anyhow::bail!("All hosts failed")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_host_port_with_valid_port() {
        let (host, port) = ProxmoxClient::parse_host_port("192.168.1.1:9000", 8006);
        assert_eq!(host, "192.168.1.1");
        assert_eq!(port, 9000);
    }

    #[test]
    fn test_parse_host_port_without_port() {
        let (host, port) = ProxmoxClient::parse_host_port("192.168.1.1", 8006);
        assert_eq!(host, "192.168.1.1");
        assert_eq!(port, 8006);
    }

    #[test]
    fn test_parse_host_port_with_invalid_port() {
        let (host, port) = ProxmoxClient::parse_host_port("192.168.1.1:invalid", 8006);
        assert_eq!(host, "192.168.1.1:invalid");
        assert_eq!(port, 8006);
    }

    #[test]
    fn test_parse_host_port_with_hostname() {
        let (host, port) = ProxmoxClient::parse_host_port("pve.example.com:8007", 8006);
        assert_eq!(host, "pve.example.com");
        assert_eq!(port, 8007);
    }

    #[test]
    fn test_parse_host_port_with_empty_port() {
        let (host, port) = ProxmoxClient::parse_host_port("192.168.1.1:", 8006);
        assert_eq!(host, "192.168.1.1:");
        assert_eq!(port, 8006);
    }

    #[test]
    fn test_new_creates_correct_base_url() {
        let client = ProxmoxClient::new("192.168.1.100", 8006, None, false).unwrap();
        assert_eq!(client.base_url, "https://192.168.1.100:8006/api2/json");
        assert!(client.token.is_none());
    }

    #[test]
    fn test_new_with_token() {
        let token = "root@pam!backup=test-token";
        let client = ProxmoxClient::new("pve.local", 8006, Some(token.to_string()), false).unwrap();
        assert_eq!(client.base_url, "https://pve.local:8006/api2/json");
        assert_eq!(client.token, Some(token.to_string()));
    }

    #[test]
    fn test_new_with_custom_port() {
        let client = ProxmoxClient::new("10.0.0.1", 9006, None, true).unwrap();
        assert_eq!(client.base_url, "https://10.0.0.1:9006/api2/json");
    }
}
