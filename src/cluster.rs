use anyhow::Result;
use serde::Deserialize;

use crate::client::ProxmoxClient;

pub struct ClusterManager {
    client: ProxmoxClient,
}

impl ClusterManager {
    pub fn new(client: ProxmoxClient) -> Self {
        Self { client }
    }

    pub async fn find_vm_node(&self, vm_identifier: &str) -> Result<(String, u32)> {
        #[derive(Deserialize)]
        struct Resource {
            node: String,
            vmid: u32,
            name: Option<String>,
            #[serde(rename = "type")]
            _resource_type: String,
        }

        let resources: Vec<Resource> = self.client.get("/cluster/resources?type=vm").await?;

        if let Ok(vmid) = vm_identifier.parse::<u32>() {
            if let Some(resource) = resources.iter().find(|r| r.vmid == vmid) {
                return Ok((resource.node.clone(), resource.vmid));
            }
        }

        if let Some(resource) = resources
            .iter()
            .find(|r| r.name.as_ref().is_some_and(|n| n == vm_identifier))
        {
            return Ok((resource.node.clone(), resource.vmid));
        }

        anyhow::bail!("VM '{}' not found in cluster", vm_identifier)
    }

    pub async fn list_nodes(&self) -> Result<()> {
        #[derive(Deserialize)]
        struct Node {
            node: String,
            status: String,
            #[serde(default)]
            _cpu: Option<f64>,
            #[serde(default)]
            _maxcpu: Option<u32>,
            #[serde(default)]
            _mem: Option<u64>,
            #[serde(default)]
            _maxmem: Option<u64>,
            #[serde(default)]
            _uptime: Option<u64>,
        }

        match self.client.get::<Vec<Node>>("/nodes").await {
            Ok(nodes) => {
                println!("Cluster nodes:");
                for node in &nodes {
                    println!("- {} ({})", node.node, node.status);
                }
                Ok(())
            }
            Err(_) => {
                // Fallback to cluster/status endpoint
                #[derive(Deserialize)]
                struct NodeInfo {
                    node: Option<String>,
                    name: Option<String>,
                    #[serde(rename = "type")]
                    node_type: String,
                    status: Option<String>,
                }

                let items: Vec<NodeInfo> = self.client.get("/cluster/status").await?;

                println!("Cluster nodes:");
                for item in items.iter().filter(|n| n.node_type == "node") {
                    if let Some(node_name) = item.node.as_ref().or(item.name.as_ref()) {
                        let status = item.status.as_deref().unwrap_or("unknown");
                        println!("- {} ({})", node_name, status);
                    }
                }
                Ok(())
            }
        }
    }
}
