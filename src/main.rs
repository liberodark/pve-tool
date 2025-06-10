use anyhow::Result;
use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::fs;

mod client;
mod cluster;
mod snapshot;

use client::ProxmoxClient;
use cluster::ClusterManager;
use snapshot::SnapshotManager;

#[derive(Debug, Deserialize, Default)]
struct Config {
    host: Option<String>,
    port: Option<u16>,
    token: Option<String>,
    node: Option<String>,
    verify_ssl: Option<bool>,
}

#[derive(Parser)]
#[command(name = "pve-tool")]
#[command(about = "Proxmox VE snapshot management tool", version)]
struct Cli {
    #[arg(short = 'c', long, help = "Path to configuration file")]
    config: Option<String>,

    #[arg(short = 'H', long, env = "PROXMOX_HOST", default_value = "192.168.1.1")]
    host: String,

    #[arg(short = 'p', long, env = "PROXMOX_PORT", default_value = "8006")]
    port: u16,

    #[arg(short = 'n', long, env = "PROXMOX_NODE")]
    node: Option<String>,

    #[arg(short = 't', long, env = "PROXMOX_API_TOKEN")]
    token: Option<String>,

    #[arg(short = 'k', long, env = "PROXMOX_VERIFY_SSL")]
    verify_ssl: Option<bool>,

    #[arg(short = 'R', long)]
    raw: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Create {
        vm: String,
        #[arg(short = 's', long)]
        snapname: Option<String>,
        #[arg(short = 'd', long)]
        description: Option<String>,
        #[arg(short = 'm', long)]
        vmstate: bool,
    },
    Delete {
        vm: String,
        snapname: String,
    },
    List {
        vm: String,
    },
    Rollback {
        vm: String,
        snapname: String,
    },
    Info {
        vm: String,
    },
    Check {
        vm: String,
    },
    Test,
    ListVms {
        #[arg(short = 'N', long)]
        node: Option<String>,
    },
    ListNodes,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut cli = Cli::parse();

    if let Some(config_path) = &cli.config {
        if let Ok(config_str) = fs::read_to_string(config_path) {
            if let Ok(config) = toml::from_str::<Config>(&config_str) {
                if cli.host == "192.168.1.1" && std::env::var("PROXMOX_HOST").is_err() {
                    if let Some(host) = config.host {
                        cli.host = host;
                    }
                }

                if cli.port == 8006 && std::env::var("PROXMOX_PORT").is_err() {
                    if let Some(port) = config.port {
                        cli.port = port;
                    }
                }

                if cli.token.is_none() && std::env::var("PROXMOX_API_TOKEN").is_err() {
                    cli.token = config.token;
                }

                if cli.node.is_none() && std::env::var("PROXMOX_NODE").is_err() {
                    cli.node = config.node;
                }

                if cli.verify_ssl.is_none() && std::env::var("PROXMOX_VERIFY_SSL").is_err() {
                    cli.verify_ssl = config.verify_ssl;
                }
            }
        }
    }

    if cli.token.is_none() {
        eprintln!(
            "Error: API token is required. Set PROXMOX_API_TOKEN, use -t, or add to config file"
        );
        std::process::exit(1);
    }

    let verify_ssl = cli.verify_ssl.unwrap_or(false);
    let client = ProxmoxClient::new(&cli.host, cli.port, cli.token, verify_ssl)?;
    let snapshot_mgr = SnapshotManager::new(client.clone());

    match cli.command {
        Commands::Create {
            vm,
            snapname,
            description,
            vmstate,
        } => {
            snapshot_mgr
                .create_snapshot(&vm, snapname, description, vmstate)
                .await?;
        }
        Commands::Delete { vm, snapname } => {
            snapshot_mgr.delete_snapshot(&vm, &snapname).await?;
        }
        Commands::List { vm } => {
            snapshot_mgr.list_snapshots(&vm).await?;
        }
        Commands::Rollback { vm, snapname } => {
            snapshot_mgr.rollback_snapshot(&vm, &snapname).await?;
        }
        Commands::Info { vm } => {
            snapshot_mgr.show_vm_info(&vm).await?;
        }
        Commands::Check { vm } => {
            snapshot_mgr.check_vm_status(&vm).await?;
        }
        Commands::Test => {
            test_connection(client).await?;
        }
        Commands::ListVms { node } => {
            snapshot_mgr.list_vms(node.as_deref()).await?;
        }
        Commands::ListNodes => {
            let cluster = ClusterManager::new(client);
            cluster.list_nodes().await?;
        }
    }

    Ok(())
}

async fn test_connection(client: ProxmoxClient) -> Result<()> {
    println!("Testing connection to Proxmox server...");

    match client.get::<serde_json::Value>("/version").await {
        Ok(version) => {
            println!("✓ Connection successful!");
            if let Some(ver) = version.get("version").and_then(|v| v.as_str()) {
                println!("  Proxmox VE version: {}", ver);
            }
        }
        Err(e) => {
            eprintln!("✗ Connection failed: {}", e);
            return Err(e);
        }
    }

    Ok(())
}
