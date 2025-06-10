# pve-tool

A safe and modern tool for managing Proxmox VE snapshots across multiple nodes in a cluster.

[![Rust](https://github.com/liberodark/pve-tool/actions/workflows/rust.yml/badge.svg)](https://github.com/liberodark/pve-tool/actions/workflows/rust.yml)

## Features

- Multi-node cluster support with automatic VM discovery
- Create, delete, list and rollback snapshots
- Support for VM names or VMIDs
- Batch operations on multiple VMs
- LUKS encrypted container support
- 100% safe Rust code (no unsafe blocks)
- Simple command-line interface with environment variable support

## Prerequisites

### Required Proxmox Configuration

- Proxmox VE 6.0 or higher
- API token with appropriate permissions
- Network access to Proxmox API (port 8006)

### Creating an API Token

1. Go to Datacenter → Permissions → API Tokens
2. Add a new token
3. Copy the token ID and secret
4. Format: `USER@REALM!TOKENID=SECRET`

Example: `root@pam!backup=xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`

## Installation

### Via cargo
```bash
cargo install pve-tool
```

### Manual build
```bash
git clone https://github.com/liberodark/pve-tool.git
cd pve-tool
cargo build --release
sudo cp target/release/pve-tool /usr/local/bin/
```

### Precompiled binaries
Precompiled binaries are available in the [Releases](https://github.com/liberodark/pve-tool/releases) section.

### NixOS
```nix
environment.systemPackages = with pkgs; [
  pve-tool
];
```

## Configuration

### Environment Variables

```bash
export PROXMOX_HOST="192.168.1.100"
export PROXMOX_PORT="8006"
export PROXMOX_API_TOKEN="root@pam!backup=xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
```

### Configuration File (optional)

Create `~/.config/pve-tool/config.toml`:

```toml
host = "192.168.1.100"
port = 8006
token = "root@pam!backup=xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
verify_ssl = false
timeout = 30
```

## Usage

### Create a snapshot

```bash
# By VM ID
pve-tool create 100

# By VM name
pve-tool create myvm.example.com

# With custom name and description
pve-tool create 100 -s daily-backup -d "Daily backup before maintenance"

# Including VM state
pve-tool create myvm -s important -d "Before upgrade" -m
```

### List snapshots

```bash
pve-tool list 100
pve-tool list myvm.example.com
```

### Delete a snapshot

```bash
pve-tool delete 100 daily-backup
pve-tool delete myvm snapshot-20240115
```

### Rollback to a snapshot

```bash
pve-tool rollback 100 daily-backup
pve-tool rollback myvm snapshot-20240115
```

### List VMs in cluster

```bash
# All VMs
pve-tool list-vms

# VMs on specific node
pve-tool list-vms -N pve1
```

### List cluster nodes

```bash
pve-tool list-nodes
```

### VM information

```bash
pve-tool info 100
pve-tool check myvm
```

### Test connection

```bash
pve-tool test
```

### Options
- `-H, --host HOST`: Proxmox server (default: from env or 192.168.1.1)
- `-p, --port PORT`: Server port (default: 8006)
- `-t, --token TOKEN`: API token
- `-v, --verbose`: Enable verbose output

## Troubleshooting

### Connection Issues
If you encounter connection errors:
- Verify the Proxmox host is reachable: `ping $PROXMOX_HOST`
- Check if the API port is open: `nc -zv $PROXMOX_HOST 8006`
- Ensure your API token is valid
- Try the test command: `pve-tool test`

### Permission Denied
Make sure your API token has the necessary permissions:
- VM.Audit (for listing)
- VM.Snapshot (for creating/deleting snapshots)
- VM.PowerMgmt (for rollback operations)

### VM Not Found
- Verify the VM exists: `pve-tool list-vms`
- Check if the VM is on a different node in the cluster
- Try using the VMID instead of the name

### SSL Certificate Errors
If using self-signed certificates, the tool automatically disables certificate verification. For production use, consider using valid certificates.

## License

This project is distributed under the [GPL-3.0](LICENSE) license.
