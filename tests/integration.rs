use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

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

#[test]
fn test_user_scope_plugin_with_local_override_is_disabled_in_cwd() {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();

    // Set up the fake user home: install + enable gitlab globally
    let claude_dir = home.path().join(".claude");
    let plugins_dir = claude_dir.join("plugins");
    fs::create_dir_all(&plugins_dir).unwrap();

    fs::write(
        claude_dir.join("settings.json"),
        r#"{"enabledPlugins":{"gitlab@market":true}}"#,
    )
    .unwrap();

    fs::write(
        plugins_dir.join("installed_plugins.json"),
        r#"{
            "version": 2,
            "plugins": {
                "gitlab@market": [{
                    "scope": "user",
                    "installPath": "/fake/path",
                    "version": "1.0.0",
                    "installedAt": "2026-01-01T00:00:00Z",
                    "lastUpdated": "2026-01-01T00:00:00Z"
                }]
            }
        }"#,
    )
    .unwrap();

    // Project local override: disabled
    let project_claude = project.path().join(".claude");
    fs::create_dir_all(&project_claude).unwrap();
    fs::write(
        project_claude.join("settings.local.json"),
        r#"{"enabledPlugins":{"gitlab@market":false}}"#,
    )
    .unwrap();

    // Run `ccpm list --debug` in project; assert effective state
    let output = Command::cargo_bin("ccpm")
        .unwrap()
        .args(["list", "--debug"])
        .env("HOME", home.path())
        .current_dir(project.path())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "ccpm list failed: {}", stderr);
    assert!(
        stderr.contains("enabled_local=Some(false)") || stderr.contains("local=Some(false)"),
        "debug output should show local override; got:\n{}",
        stderr
    );
    assert!(
        stdout.contains("disabled"),
        "gitlab should appear disabled in list output; got:\n{}",
        stdout
    );
}

#[test]
fn test_cli_disable_with_scope_local_writes_local_settings() {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();

    // Minimal fake user home so ccpm can run
    fs::create_dir_all(home.path().join(".claude/plugins")).unwrap();
    fs::write(
        home.path().join(".claude/plugins/installed_plugins.json"),
        r#"{"version":2,"plugins":{}}"#,
    )
    .unwrap();

    let output = Command::cargo_bin("ccpm")
        .unwrap()
        .args(["disable", "demo@market", "--scope", "local"])
        .env("HOME", home.path())
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "disable command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let local_settings = project.path().join(".claude/settings.local.json");
    assert!(local_settings.exists(), "settings.local.json not created");

    let content = fs::read_to_string(&local_settings).unwrap();
    assert!(
        content.contains("\"demo@market\""),
        "settings.local.json missing entry: {}",
        content
    );
    assert!(
        content.contains("false"),
        "settings.local.json should set demo@market to false: {}",
        content
    );
}

#[test]
fn test_cli_info_shows_override_source_path() {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();

    // Fake user home: gitlab installed user-scope, enabled globally
    let claude_dir = home.path().join(".claude");
    let plugins_dir = claude_dir.join("plugins");
    fs::create_dir_all(&plugins_dir).unwrap();

    fs::write(
        claude_dir.join("settings.json"),
        r#"{"enabledPlugins":{"gitlab@market":true}}"#,
    )
    .unwrap();

    fs::write(
        plugins_dir.join("installed_plugins.json"),
        r#"{
            "version": 2,
            "plugins": {
                "gitlab@market": [{
                    "scope": "user",
                    "installPath": "/fake/path",
                    "version": "1.0.0",
                    "installedAt": "2026-01-01T00:00:00Z",
                    "lastUpdated": "2026-01-01T00:00:00Z"
                }]
            }
        }"#,
    )
    .unwrap();

    // Project local override: disabled
    let project_claude = project.path().join(".claude");
    fs::create_dir_all(&project_claude).unwrap();
    fs::write(
        project_claude.join("settings.local.json"),
        r#"{"enabledPlugins":{"gitlab@market":false}}"#,
    )
    .unwrap();

    let output = Command::cargo_bin("ccpm")
        .unwrap()
        .args(["info", "gitlab@market"])
        .env("HOME", home.path())
        .current_dir(project.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "ccpm info failed: {stderr}");

    // Settings block must include a Local row showing the disabled value
    assert!(
        stdout.contains("Local:") && stdout.contains("disabled"),
        "info should show 'Local: disabled' row; got:\n{stdout}"
    );
    // The Local row should be annotated with the source file path
    assert!(
        stdout.contains(" · ") && stdout.contains("settings.local.json"),
        "info should annotate Local row with ' · <path>/settings.local.json'; got:\n{stdout}"
    );
}
