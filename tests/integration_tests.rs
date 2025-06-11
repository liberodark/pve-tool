use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_help_command() {
    let mut cmd = Command::cargo_bin("pve-tool").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Proxmox VE snapshot management tool",
        ));
}

#[test]
fn test_version_command() {
    let mut cmd = Command::cargo_bin("pve-tool").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("pve-tool"));
}

#[test]
fn test_missing_token() {
    let mut cmd = Command::cargo_bin("pve-tool").unwrap();
    cmd.arg("test")
        .env_remove("PROXMOX_API_TOKEN")
        .assert()
        .failure()
        .stderr(predicate::str::contains("API token is required"));
}

#[test]
fn test_config_file_parsing() {
    let mut config_file = NamedTempFile::new().unwrap();
    writeln!(
        config_file,
        r#"
host = "test.example.com"
port = 8007
token = "test-token"
verify_ssl = false
"#
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("pve-tool").unwrap();
    cmd.arg("--config")
        .arg(config_file.path())
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn test_create_subcommand_structure() {
    let mut cmd = Command::cargo_bin("pve-tool").unwrap();
    cmd.arg("create")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--snapname"))
        .stdout(predicate::str::contains("--description"))
        .stdout(predicate::str::contains("--vmstate"));
}

#[test]
fn test_environment_variables_parsing() {
    let mut cmd = Command::cargo_bin("pve-tool").unwrap();
    cmd.env("PROXMOX_HOST", "env.example.com")
        .env("PROXMOX_PORT", "8008")
        .env("PROXMOX_API_TOKEN", "env-token")
        .env("PROXMOX_VERIFY_SSL", "true")
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn test_list_vms_help() {
    let mut cmd = Command::cargo_bin("pve-tool").unwrap();
    cmd.arg("list-vms")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--node"));
}

#[test]
fn test_invalid_subcommand() {
    let mut cmd = Command::cargo_bin("pve-tool").unwrap();
    cmd.arg("invalid-command")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

#[test]
fn test_all_subcommands_help() {
    let subcommands = vec![
        "create",
        "delete",
        "list",
        "rollback",
        "info",
        "check",
        "test",
        "list-vms",
        "list-nodes",
    ];

    for subcommand in subcommands {
        let mut cmd = Command::cargo_bin("pve-tool").unwrap();
        cmd.arg(subcommand).arg("--help").assert().success();
    }
}

#[test]
fn test_cluster_config_parsing() {
    let mut config_file = NamedTempFile::new().unwrap();
    writeln!(
        config_file,
        r#"
[clusters.prod]
hosts = ["192.168.1.100", "192.168.1.101"]
token = "prod-token"
verify_ssl = true

[clusters.dev]
hosts = ["192.168.2.100"]
port = 8007
token = "dev-token"
"#
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("pve-tool").unwrap();
    cmd.arg("--config")
        .arg(config_file.path())
        .arg("--cluster")
        .arg("dev")
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn test_required_arguments() {
    let mut cmd = Command::cargo_bin("pve-tool").unwrap();
    cmd.env("PROXMOX_API_TOKEN", "test-token")
        .arg("delete")
        .arg("100")
        .assert()
        .failure();

    let mut cmd = Command::cargo_bin("pve-tool").unwrap();
    cmd.env("PROXMOX_API_TOKEN", "test-token")
        .arg("rollback")
        .arg("100")
        .assert()
        .failure();
}
