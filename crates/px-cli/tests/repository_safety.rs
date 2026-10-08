//! Deterministic command-boundary regressions, independent of a live server.
#![cfg(unix)]
use assert_cmd::Command;
use predicates::prelude::*;
use std::{fs, os::unix::fs::PermissionsExt, path::Path};
use tempfile::TempDir;

fn checkout(home: &Path, name: &str, server: &str) {
    let root = home.join(name);
    fs::create_dir_all(root.join(".lore")).unwrap();
    fs::create_dir(root.join("character")).unwrap();
    fs::write(
        root.join("repository.yaml"),
        format!("id: px://{name}/world/{name}\n"),
    )
    .unwrap();
    fs::write(
        root.join("character/hero.yaml"),
        format!("id: px://{name}/character/hero\n"),
    )
    .unwrap();
    fs::write(
        root.join(".lore/config.toml"),
        format!("remote_url = '{server}'\n[file]\ndirect_io = false\n"),
    )
    .unwrap();
}

fn fake_lore(temp: &TempDir, body: &str) -> std::path::PathBuf {
    let file = temp.path().join("lore-fixture");
    fs::write(&file, format!("#!/bin/sh\nif [ \"$1\" = repository ] && [ \"$2\" = info ]; then\n  printf '%s\\n' '{{\"tagName\":\"repositoryData\",\"data\":{{\"id\":\"00000000000000000000000000000001\",\"remoteUrl\":\"lore://server-a:41337\"}}}}'\n  exit 0\nfi\n{body}\n")).unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).unwrap();
    file
}

#[test]
fn entity_pull_and_push_use_checkout_server_and_leave_sibling_unchanged() {
    let temp = TempDir::new().unwrap();
    checkout(temp.path(), "bears", "lore://server-a:41337");
    checkout(temp.path(), "toystory", "lore://toys:41337");
    fs::write(temp.path().join("provider.toml"), "provider_type = 'remote'\nremote_url = 'lore://server-b:41337'\nworkspace_id = 'default'\n").unwrap();
    let sibling = fs::read(temp.path().join("toystory/.lore/config.toml")).unwrap();
    let fake = fake_lore(
        &temp,
        "pwd >> \"$PX_TEST_CALLS\"\nprintf '%s\\n' \"$*\" >> \"$PX_TEST_CALLS\"\ncat .lore/config.toml >> \"$PX_TEST_CALLS\"",
    );
    let log = temp.path().join("calls");
    for args in [
        vec!["pull", "px://bears/character/hero"],
        vec!["pull", "bears/character/hero"],
        vec!["push", "bears", "--branch", "classic"],
    ] {
        Command::cargo_bin("px")
            .unwrap()
            .args(args)
            .arg("--base-dir")
            .arg(temp.path())
            .env("PXLORE_CLI", &fake)
            .env("PX_TEST_CALLS", &log)
            .assert()
            .success();
    }
    let calls = fs::read_to_string(log).unwrap();
    assert!(
        calls.contains("revision sync --root-file repository.yaml --root-file character/hero.yaml")
    );
    assert!(calls.contains("branch push classic"));
    assert!(calls.contains("lore://server-a:41337"));
    assert!(!calls.contains("server-b"));
    assert!(!calls.contains("toystory"));
    assert_eq!(
        fs::read(temp.path().join("toystory/.lore/config.toml")).unwrap(),
        sibling
    );
    assert!(!temp.path().join("toystory/.loreignore").exists());
}

#[test]
fn repository_override_reaches_lore_without_losing_settings() {
    let temp = TempDir::new().unwrap();
    checkout(temp.path(), "bears", "lore://server-a:41337");
    Command::cargo_bin("px")
        .unwrap()
        .args(["remote", "set", "bears", "lores://custom.example/bears"])
        .arg("--base-dir")
        .arg(temp.path())
        .assert()
        .success();
    let config = fs::read_to_string(temp.path().join("bears/.lore/config.toml")).unwrap();
    assert!(config.contains("lores://custom.example"));
    assert!(config.contains("direct_io = false"));
}

#[test]
fn configure_preserves_pinned_remotes_and_force_migrates_them() {
    let temp = TempDir::new().unwrap();
    checkout(temp.path(), "bears", "lore://server-a:41337");
    fs::write(temp.path().join("provider.toml"), "provider_type = 'remote'\nremote_url = 'lore://server-a:41337'\nworkspace_id = 'default'\n").unwrap();
    let old = fs::read(temp.path().join("bears/.lore/config.toml")).unwrap();
    Command::cargo_bin("px")
        .unwrap()
        .args([
            "configure",
            "remote",
            "--remote-url",
            "lore://server-b:41337",
            "--no-initial-commit",
        ])
        .arg("--base-dir")
        .arg(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Not updated"));
    assert_eq!(
        fs::read(temp.path().join("bears/.lore/config.toml")).unwrap(),
        old
    );
    Command::cargo_bin("px")
        .unwrap()
        .args([
            "configure",
            "remote",
            "--remote-url",
            "lore://server-b:41337",
            "--no-initial-commit",
            "--force",
        ])
        .arg("--base-dir")
        .arg(temp.path())
        .assert()
        .success();
    assert!(
        fs::read_to_string(temp.path().join("bears/.lore/config.toml"))
            .unwrap()
            .contains("server-b")
    );
    assert!(
        fs::read_to_string(temp.path().join("bears/.px/config.yaml"))
            .unwrap()
            .contains("remote_source: default")
    );
    // Default-managed repositories follow the next provider update.
    Command::cargo_bin("px")
        .unwrap()
        .args([
            "configure",
            "remote",
            "--remote-url",
            "lore://server-c:41337",
            "--no-initial-commit",
        ])
        .arg("--base-dir")
        .arg(temp.path())
        .assert()
        .success();
    assert!(
        fs::read_to_string(temp.path().join("bears/.lore/config.toml"))
            .unwrap()
            .contains("server-c")
    );
}

#[test]
fn lore_diagnostics_are_clean_by_default_and_complete_with_verbose_in_json() {
    let temp = TempDir::new().unwrap();
    checkout(temp.path(), "bears", "lore://server-a:41337");
    let fake = fake_lore(
        &temp,
        "printf 'Error: transport error\\nStack backtrace:\\n 0: /build/lore/internal.rs:10\\n' >&2\nexit 1",
    );
    for verbose in [false, true] {
        let mut cmd = Command::cargo_bin("px").unwrap();
        cmd.args(["push", "bears", "--branch", "main"])
            .arg("--base-dir")
            .arg(temp.path())
            .env("PXLORE_CLI", &fake);
        if verbose {
            cmd.arg("--verbose");
        }
        let output = cmd.output().unwrap();
        assert!(!output.status.success());
        let stderr = String::from_utf8(output.stderr).unwrap();
        let error: serde_json::Value =
            serde_json::from_str(stderr.lines().last().unwrap()).unwrap();
        let message = error["error"].as_str().unwrap();
        assert!(message.contains(if verbose {
            "/build/lore/internal.rs:10"
        } else {
            "check its remote URL"
        }));
        assert_eq!(message.contains("Stack backtrace"), verbose);
    }
}

#[test]
fn ignored_system_files_are_present_in_lore_filter_before_sync() {
    let temp = TempDir::new().unwrap();
    checkout(temp.path(), "bears", "lore://server-a:41337");
    fs::write(temp.path().join("bears/.pxignore"), "cache/\n*.scratch\n").unwrap();
    fs::write(temp.path().join("bears/.DS_Store"), "local Finder metadata").unwrap();
    let fake = fake_lore(
        &temp,
        "test -f .loreignore || exit 1\ncat .loreignore > \"$PX_TEST_CALLS\"",
    );
    Command::cargo_bin("px")
        .unwrap()
        .args(["pull", "bears/character/hero"])
        .arg("--base-dir")
        .arg(temp.path())
        .env("PXLORE_CLI", fake)
        .env("PX_TEST_CALLS", temp.path().join("ignore-at-sync"))
        .assert()
        .success();
    let filter = fs::read_to_string(temp.path().join("ignore-at-sync")).unwrap();
    assert!(filter.contains(".DS_Store\n"));
    assert!(filter.contains("cache/\n"));
    assert!(
        temp.path().join("bears/.DS_Store").exists(),
        "filtering must not destroy local files"
    );
}

#[test]
fn missing_remote_content_is_actionable_even_with_json_stdout_and_stderr_logs() {
    let temp = TempDir::new().unwrap();
    checkout(temp.path(), "bears", "lore://server-a:41337");
    let fake = fake_lore(
        &temp,
        r#"printf '%s\n' '{"tagName":"complete","data":{"error":{"message":"content address not found","traceLocations":["/build/lore/internal.rs:10"]}}}'
printf '%s\n' 'internal operation failed at /build/store.rs:10' >&2
exit 1"#,
    );
    for verbose in [false, true] {
        let mut cmd = Command::cargo_bin("px").unwrap();
        cmd.args(["pull", "bears/character/hero"])
            .arg("--base-dir")
            .arg(temp.path())
            .env("PXLORE_CLI", &fake);
        if verbose {
            cmd.arg("--verbose");
        }
        let output = cmd.output().unwrap();
        assert!(!output.status.success());
        let stderr = String::from_utf8(output.stderr).unwrap();
        let error: serde_json::Value =
            serde_json::from_str(stderr.lines().last().unwrap()).unwrap();
        let message = error["error"].as_str().unwrap();
        assert!(message.contains("commit and push"));
        assert_eq!(message.contains("/build/lore/internal.rs:10"), verbose);
        assert_eq!(message.contains("/build/store.rs:10"), verbose);
    }
}
