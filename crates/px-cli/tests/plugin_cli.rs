use assert_cmd::Command;
use predicates::prelude::*;

#[cfg(any(
    all(target_os = "macos", target_arch = "aarch64"),
    all(target_os = "linux", target_arch = "x86_64")
))]
#[test]
fn upgrade_preserves_disabled_state_and_recovers_an_interrupted_switch() {
    use px_plugin::registry::Store;
    use std::fs;
    let root = tempfile::tempdir().unwrap();
    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/hello");
    let candidate = root.path().join("candidate");
    fs::create_dir_all(candidate.join("bin")).unwrap();
    fs::copy(source.join("bin/hello"), candidate.join("bin/hello")).unwrap();
    let manifest = fs::read_to_string(source.join("plugin.yaml"))
        .unwrap()
        .replace("version: 0.1.0", "version: 0.2.0");
    fs::write(candidate.join("plugin.yaml"), manifest).unwrap();
    let home = root.path().join("px");
    let store = Store::new(&home);
    store.install_directory(&source).unwrap();
    let platform = if cfg!(target_os = "macos") {
        "aarch64-apple-darwin"
    } else {
        "x86_64-unknown-linux-gnu"
    };
    store
        .upgrade_directory(&candidate, "0.10.0", platform)
        .unwrap();
    assert_eq!(
        store.load().unwrap().installed["example.hello"].version,
        "0.2.0"
    );
    Command::new(assert_cmd::cargo::cargo_bin!("px"))
        .arg("--base-dir")
        .arg(&home)
        .env("HOME", root.path())
        .args(["plugin", "list"])
        .assert()
        .success();
    let restored = store.load().unwrap();
    assert_eq!(restored.installed["example.hello"].version, "0.1.0");
    assert!(!restored.installed["example.hello"].enabled);
    assert!(store.pending_upgrade().unwrap().is_none());
}

#[cfg(any(
    all(target_os = "macos", target_arch = "aarch64"),
    all(target_os = "linux", target_arch = "x86_64")
))]
#[test]
fn upgrade_rolls_back_when_a_skill_belongs_to_the_user() {
    use px_plugin::registry::Store;
    use std::fs;
    let root = tempfile::tempdir().unwrap();
    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/hello");
    let candidate = root.path().join("candidate");
    fs::create_dir_all(candidate.join("bin")).unwrap();
    fs::create_dir_all(candidate.join("skills/conflict")).unwrap();
    fs::copy(source.join("bin/hello"), candidate.join("bin/hello")).unwrap();
    fs::write(
        candidate.join("skills/conflict/SKILL.md"),
        "new plugin skill",
    )
    .unwrap();
    let manifest = fs::read_to_string(source.join("plugin.yaml"))
        .unwrap()
        .replace("version: 0.1.0", "version: 0.2.0")
        .replace("skills: []", "skills: [skills/conflict]");
    fs::write(candidate.join("plugin.yaml"), manifest).unwrap();
    let user_skill = root.path().join(".agents/skills/conflict");
    fs::create_dir_all(&user_skill).unwrap();
    fs::write(user_skill.join("SKILL.md"), "user content").unwrap();
    let home = root.path().join("px");
    let store = Store::new(&home);
    store.install_directory(&source).unwrap();
    store.set_enabled("example.hello", true).unwrap();
    Command::new(assert_cmd::cargo::cargo_bin!("px"))
        .arg("--base-dir")
        .arg(&home)
        .env("HOME", root.path())
        .args(["plugin", "upgrade", candidate.to_str().unwrap()])
        .assert()
        .failure();
    let installed = store.load().unwrap();
    assert_eq!(installed.installed["example.hello"].version, "0.1.0");
    assert!(installed.installed["example.hello"].enabled);
    assert_eq!(
        fs::read_to_string(user_skill.join("SKILL.md")).unwrap(),
        "user content"
    );
    assert!(store.pending_upgrade().unwrap().is_none());
    fs::remove_dir_all(&user_skill).unwrap();
    Command::new(assert_cmd::cargo::cargo_bin!("px"))
        .arg("--base-dir")
        .arg(&home)
        .env("HOME", root.path())
        .args(["plugin", "upgrade", candidate.to_str().unwrap()])
        .assert()
        .success();
    let upgraded = store.load().unwrap();
    assert_eq!(upgraded.installed["example.hello"].version, "0.2.0");
    assert!(upgraded.installed["example.hello"].enabled);
    assert_eq!(
        fs::read_to_string(user_skill.join(".px-owner")).unwrap(),
        "example.hello"
    );
}

#[test]
fn installs_enables_disables_and_uninstalls_local_plugin() {
    let home = tempfile::tempdir().unwrap();
    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/hello");
    let run = |args: &[&str]| {
        let mut command = Command::new(assert_cmd::cargo::cargo_bin!("px"));
        command.arg("--base-dir").arg(home.path()).args(args);
        command
    };
    run(&["plugin", "install", source.to_str().unwrap()])
        .assert()
        .success();
    run(&["plugin", "enable", "example.hello"])
        .assert()
        .success();
    run(&["plugin", "tools", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("px_hello_greet"));
    run(&[
        "plugin",
        "call",
        "example.hello",
        "greet",
        "--input",
        r#"{"name":"PX"}"#,
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Hello, PX!"));
    let config_dir = home.path().join("project/.px");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(config_dir.join("config.yaml"), "plugins: {}\n").unwrap();
    run(&[
        "plugin",
        "call",
        "example.hello",
        "greet",
        "--input",
        r#"{"name":"PX","repository":"project"}"#,
    ])
    .assert()
    .failure();
    std::fs::write(
        config_dir.join("config.yaml"),
        "plugins:\n  example.hello: 0.1.0\n",
    )
    .unwrap();
    run(&[
        "plugin",
        "call",
        "example.hello",
        "greet",
        "--input",
        r#"{"name":"PX","repository":"project"}"#,
    ])
    .assert()
    .success();
    run(&["plugin", "call", "example.hello", "greet", "--input", "{}"])
        .assert()
        .failure();
    run(&["plugin", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("example.hello"));
    run(&["plugin", "info", "example.hello"]).assert().success();
    run(&["plugin", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("example.hello"));
    run(&["plugin", "disable", "example.hello"])
        .assert()
        .success();
    run(&["plugin", "tools"])
        .assert()
        .success()
        .stdout(predicate::str::contains("px_hello_greet").not());
    run(&["plugin", "uninstall", "example.hello"])
        .assert()
        .success();
    run(&["plugin", "info", "example.hello"]).assert().failure();
    run(&["schema", "plugin"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PluginManifest"));
}

#[test]
fn repository_enable_disable_updates_pins_and_preserves_other_configuration() {
    let home = tempfile::tempdir().unwrap();
    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/hello");
    let run = |args: &[&str]| {
        let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("px"));
        cmd.arg("--base-dir").arg(home.path()).args(args);
        cmd
    };
    run(&["plugin", "install", source.to_str().unwrap()])
        .assert()
        .success();
    run(&["plugin", "enable", "example.hello"])
        .assert()
        .success();
    let config = home.path().join("demo/.px/config.yaml");
    std::fs::create_dir_all(config.parent().unwrap()).unwrap();
    std::fs::write(&config, "custom: keep\n").unwrap();
    run(&["plugin", "disable", "example.hello", "--repo", "demo"])
        .assert()
        .success();
    run(&[
        "plugin",
        "call",
        "example.hello",
        "greet",
        "--input",
        r#"{"name":"PX","repository":"demo"}"#,
    ])
    .assert()
    .failure();
    run(&["plugin", "enable", "example.hello", "--repo", "demo"])
        .assert()
        .success();
    run(&[
        "plugin",
        "call",
        "example.hello",
        "greet",
        "--input",
        r#"{"name":"PX","repository":"demo"}"#,
    ])
    .assert()
    .success();
    let value: serde_yaml::Value =
        serde_yaml::from_str(&std::fs::read_to_string(config).unwrap()).unwrap();
    assert_eq!(value["custom"].as_str(), Some("keep"));
    assert_eq!(value["plugins"]["example.hello"].as_str(), Some("0.1.0"));
    run(&["plugin", "enable", "example.hello", "--repo", "../unsafe"])
        .assert()
        .failure();
}
