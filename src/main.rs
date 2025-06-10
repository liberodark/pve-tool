use anyhow::Result;
use clap::{Parser, Subcommand};
use std::fs;

mod client;
mod cluster;
mod config;
mod snapshot;

use client::ProxmoxClient;
use cluster::ClusterManager;
use config::Config;
use snapshot::SnapshotManager;

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

    #[arg(long, help = "Cluster name from config file")]
    cluster: Option<String>,

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

    let mut config = Config::default();
    if let Some(config_path) = &cli.config {
        if let Ok(config_str) = fs::read_to_string(config_path) {
            config = toml::from_str::<Config>(&config_str)?;
        }
    }

    if cli.host == "192.168.1.1" && std::env::var("PROXMOX_HOST").is_err() {
        if let Some(host) = &config.host {
            cli.host = host.clone();
        }
    }

    if cli.port == 8006 && std::env::var("PROXMOX_PORT").is_err() {
        if let Some(port) = config.port {
            cli.port = port;
        }
    }

    if cli.token.is_none() && std::env::var("PROXMOX_API_TOKEN").is_err() {
        cli.token = config.token.clone();
    }

    if cli.node.is_none() && std::env::var("PROXMOX_NODE").is_err() {
        cli.node = config.node.clone();
    }

    if cli.verify_ssl.is_none() && std::env::var("PROXMOX_VERIFY_SSL").is_err() {
        cli.verify_ssl = config.verify_ssl;
    }

    let client = if let Some(cluster_config) = config.get_cluster(cli.cluster.as_deref()) {
        let port = cluster_config.port.unwrap_or(cli.port);
        let token = cluster_config.token.or(cli.token.clone());
        let verify_ssl = cluster_config
            .verify_ssl
            .unwrap_or(cli.verify_ssl.unwrap_or(false));

        if cluster_config.hosts.is_empty() {
            anyhow::bail!("No hosts configured for cluster");
        }

        ProxmoxClient::new_with_fallback(&cluster_config.hosts, port, token, verify_ssl).await?
    } else {
        if cli.token.is_none() {
            eprintln!(
                "Error: API token is required. Set PROXMOX_API_TOKEN, use -t, or add to config file"
            );
            std::process::exit(1);
        }

        let verify_ssl = cli.verify_ssl.unwrap_or(false);
        ProxmoxClient::new(&cli.host, cli.port, cli.token.clone(), verify_ssl)?
    };

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
