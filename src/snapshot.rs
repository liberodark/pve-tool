use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, sleep};

use crate::client::ProxmoxClient;
use crate::cluster::ClusterManager;

pub struct SnapshotManager {
    client: ProxmoxClient,
    cluster: ClusterManager,
}

impl SnapshotManager {
    pub fn new(client: ProxmoxClient) -> Self {
        let cluster = ClusterManager::new(client.clone());
        Self { client, cluster }
    }

    pub async fn create_snapshot(
        &self,
        vm_identifier: &str,
        snapname: Option<String>,
        description: Option<String>,
        vmstate: bool,
    ) -> Result<()> {
        let (node, vmid) = self.cluster.find_vm_node(vm_identifier).await?;

        let snapname = snapname.unwrap_or_else(|| {
            format!("snapshot-{}", chrono::Local::now().format("%Y%m%d-%H%M%S"))
        });

        let description = description.unwrap_or_else(|| {
            format!(
                "Snapshot created on {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
            )
        });

        #[derive(Serialize)]
        struct SnapshotRequest {
            snapname: String,
            description: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            vmstate: Option<u8>,
        }

        let request = SnapshotRequest {
            snapname: snapname.clone(),
            description,
            vmstate: if vmstate { Some(1) } else { None },
        };

        let task_id: String = self
            .client
            .post(&format!("/nodes/{}/qemu/{}/snapshot", node, vmid), &request)
            .await?;

        println!(
            "Creating snapshot '{}' on node {} for VM {}...",
            snapname, node, vmid
        );
        self.wait_for_task(&node, &task_id).await?;

        Ok(())
    }

    pub async fn delete_snapshot(&self, vm_identifier: &str, snapname: &str) -> Result<()> {
        let (node, vmid) = self.cluster.find_vm_node(vm_identifier).await?;

        let task_id = self
            .client
            .delete(&format!(
                "/nodes/{}/qemu/{}/snapshot/{}",
                node, vmid, snapname
            ))
            .await?;

        println!(
            "Deleting snapshot '{}' on node {} for VM {}...",
            snapname, node, vmid
        );
        self.wait_for_task(&node, &task_id).await?;

        Ok(())
    }

    pub async fn list_snapshots(&self, vm_identifier: &str) -> Result<()> {
        let (node, vmid) = self.cluster.find_vm_node(vm_identifier).await?;

        #[derive(Deserialize)]
        struct Snapshot {
            name: String,
            description: Option<String>,
            snaptime: Option<i64>,
        }

        let snapshots: Vec<Snapshot> = self
            .client
            .get(&format!("/nodes/{}/qemu/{}/snapshot", node, vmid))
            .await?;

        println!("Snapshots for VM {} on node {}:", vmid, node);
        for snap in snapshots.iter().filter(|s| s.name != "current") {
            let time = snap
                .snaptime
                .map(|t| {
                    chrono::DateTime::from_timestamp(t, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| "Unknown".to_string())
                })
                .unwrap_or_else(|| "Unknown".to_string());

            println!(
                "- {} [{}] (Created: {})",
                snap.name,
                snap.description.as_deref().unwrap_or("No description"),
                time
            );
        }

        Ok(())
    }

    pub async fn rollback_snapshot(&self, vm_identifier: &str, snapname: &str) -> Result<()> {
        let (node, vmid) = self.cluster.find_vm_node(vm_identifier).await?;

        let task_id: String = self
            .client
            .post(
                &format!(
                    "/nodes/{}/qemu/{}/snapshot/{}/rollback",
                    node, vmid, snapname
                ),
                &(),
            )
            .await?;

        println!(
            "Rolling back VM {} to snapshot '{}' on node {}...",
            vmid, snapname, node
        );
        self.wait_for_task(&node, &task_id).await?;

        Ok(())
    }

    pub async fn show_vm_info(&self, vm_identifier: &str) -> Result<()> {
        let (node, vmid) = self.cluster.find_vm_node(vm_identifier).await?;

        let info: serde_json::Value = self
            .client
            .get(&format!("/nodes/{}/qemu/{}/status/current", node, vmid))
            .await?;

        println!("VM Information:");
        println!("  Node: {}", node);
        println!("  VMID: {}", vmid);

        if let Some(name) = info.get("name").and_then(|v| v.as_str()) {
            println!("  Name: {}", name);
        }

        if let Some(status) = info.get("status").and_then(|v| v.as_str()) {
            println!("  Status: {}", status);
        }

        if let Some(cpu) = info.get("cpu").and_then(|v| v.as_f64()) {
            println!("  CPU Usage: {:.2}%", cpu * 100.0);
        }

        if let Some(mem) = info.get("mem").and_then(|v| v.as_u64()) {
            if let Some(maxmem) = info.get("maxmem").and_then(|v| v.as_u64()) {
                println!(
                    "  Memory: {} MB / {} MB ({:.1}%)",
                    mem / 1048576,
                    maxmem / 1048576,
                    (mem as f64 / maxmem as f64) * 100.0
                );
            }
        }

        Ok(())
    }

    pub async fn check_vm_status(&self, vm_identifier: &str) -> Result<()> {
        let (node, vmid) = self.cluster.find_vm_node(vm_identifier).await?;

        let status: serde_json::Value = self
            .client
            .get(&format!("/nodes/{}/qemu/{}/status/current", node, vmid))
            .await?;

        let vm_status = status
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let name = status
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");

        println!("VM ID: {}", vmid);
        println!("Name: {}", name);
        println!("Node: {}", node);
        println!("Status: {}", vm_status);

        if vm_status == "running" {
            if let Some(uptime) = status.get("uptime").and_then(|v| v.as_u64()) {
                let days = uptime / 86400;
                let hours = (uptime % 86400) / 3600;
                let minutes = (uptime % 3600) / 60;
                println!("Uptime: {}d {}h {}m", days, hours, minutes);
            }
        }

        Ok(())
    }

    pub async fn list_vms(&self, node_filter: Option<&str>) -> Result<()> {
        #[derive(Deserialize)]
        struct VmResource {
            node: String,
            vmid: u32,
            name: Option<String>,
            status: String,
            #[serde(rename = "type")]
            _resource_type: String,
            _cpu: Option<f64>,
            _mem: Option<u64>,
            _maxmem: Option<u64>,
        }

        let resources: Vec<VmResource> = self.client.get("/cluster/resources?type=vm").await?;

        let filtered: Vec<_> = if let Some(node) = node_filter {
            resources.into_iter().filter(|r| r.node == node).collect()
        } else {
            resources
        };

        if filtered.is_empty() {
            println!("No VMs found");
            return Ok(());
        }

        println!("VMs in cluster:");
        println!(
            "{:<8} {:<20} {:<10} {:<10}",
            "VMID", "Name", "Node", "Status"
        );
        println!("{}", "-".repeat(50));

        for vm in filtered {
            println!(
                "{:<8} {:<20} {:<10} {:<10}",
                vm.vmid,
                vm.name.unwrap_or_else(|| "-".to_string()),
                vm.node,
                vm.status
            );
        }

        Ok(())
    }

    async fn wait_for_task(&self, node: &str, task_id: &str) -> Result<()> {
        loop {
            #[derive(Deserialize)]
            struct TaskStatus {
                status: String,
                exitstatus: Option<String>,
            }

            let status: TaskStatus = self
                .client
                .get(&format!("/nodes/{}/tasks/{}/status", node, task_id))
                .await?;

            match status.status.as_str() {
                "stopped" => {
                    if status.exitstatus.as_deref() == Some("OK") {
                        println!("\n✓ Task completed successfully");
                        return Ok(());
                    } else {
                        anyhow::bail!("Task failed: {:?}", status.exitstatus);
                    }
                }
                "running" => {
                    print!(".");
                    std::io::Write::flush(&mut std::io::stdout())?;
                    sleep(Duration::from_secs(2)).await;
                }
                _ => anyhow::bail!("Unknown task status: {}", status.status),
            }
        }
    }
}
