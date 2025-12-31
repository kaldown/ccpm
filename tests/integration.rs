use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("ccpm").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Claude Code Plugin Manager"));
}

#[test]
fn test_cli_version() {
    let mut cmd = Command::cargo_bin("ccpm").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("ccpm"));
}

#[test]
fn test_cli_list() {
    let mut cmd = Command::cargo_bin("ccpm").unwrap();
    cmd.arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("NAME"));
}

#[test]
fn test_cli_list_scope_filter() {
    let mut cmd = Command::cargo_bin("ccpm").unwrap();
    cmd.args(["list", "--scope", "user"]).assert().success();
}

#[test]
fn test_cli_list_enabled_filter() {
    let mut cmd = Command::cargo_bin("ccpm").unwrap();
    cmd.args(["list", "--enabled"]).assert().success();
}

#[test]
fn test_cli_info_not_found() {
    let mut cmd = Command::cargo_bin("ccpm").unwrap();
    cmd.args(["info", "nonexistent-plugin@fake-marketplace"])
        .assert()
        .success()
        .stdout(predicate::str::contains("not found"));
}

#[test]
fn test_cli_enable_help() {
    let mut cmd = Command::cargo_bin("ccpm").unwrap();
    cmd.args(["enable", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Enable a plugin"));
}

#[test]
fn test_cli_disable_help() {
    let mut cmd = Command::cargo_bin("ccpm").unwrap();
    cmd.args(["disable", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Disable a plugin"));
}
