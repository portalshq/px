use assert_cmd::Command;
use predicates::prelude::*;

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
