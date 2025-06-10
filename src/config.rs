use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
pub struct ClusterConfig {
    pub hosts: Vec<String>,
    pub port: Option<u16>,
    pub token: Option<String>,
    pub verify_ssl: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub token: Option<String>,
    pub node: Option<String>,
    pub verify_ssl: Option<bool>,
    pub clusters: Option<HashMap<String, ClusterConfig>>,
}

impl Config {
    pub fn get_cluster(&self, name: Option<&str>) -> Option<ClusterConfig> {
        if let Some(name) = name {
            self.clusters.as_ref()?.get(name).cloned()
        } else if let Some(host) = &self.host {
            Some(ClusterConfig {
                hosts: vec![host.clone()],
                port: self.port,
                token: self.token.clone(),
                verify_ssl: self.verify_ssl,
            })
        } else if let Some(clusters) = &self.clusters {
            clusters.values().next().cloned()
        } else {
            None
        }
    }
}
