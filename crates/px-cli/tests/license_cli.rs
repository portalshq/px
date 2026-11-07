use assert_cmd::Command;
use ed25519_dalek::{Signer, SigningKey};
use predicates::prelude::*;
use px_plugin::entitlement::{Claims, SignedToken};

#[test]
fn offline_license_commands_verify_status_add_remove_and_gate_paid_tools() {
    let home = tempfile::tempdir().unwrap();
    let source = home.path().join("package");
    std::fs::create_dir_all(source.join("bin")).unwrap();
    let example =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/hello");
    let mut manifest: px_plugin::manifest::PluginManifest =
        serde_yaml::from_str(&std::fs::read_to_string(example.join("plugin.yaml")).unwrap())
            .unwrap();
    manifest.commerce = px_plugin::manifest::Commerce::OneTime {
        product: "hello".into(),
        major: 1,
    };
    std::fs::write(
        source.join("plugin.yaml"),
        serde_yaml::to_string(&manifest).unwrap(),
    )
    .unwrap();
    std::fs::copy(example.join("bin/hello"), source.join("bin/hello")).unwrap();
    let run = |args: &[&str]| {
        let mut command = Command::new(assert_cmd::cargo::cargo_bin!("px"));
        command.arg("--base-dir").arg(home.path()).args(args);
        command
    };
    run(&["license", "status"])
        .assert()
        .success()
        .stdout("[]\n");
    run(&["plugin", "install", source.to_str().unwrap()])
        .assert()
        .success();
    run(&["plugin", "enable", "example.hello"])
        .assert()
        .success();
    run(&["plugin", "tools"])
        .assert()
        .success()
        .stdout(predicate::str::contains("px_hello_greet").not());
    let key = SigningKey::from_bytes(&[31; 32]);
    let claims = Claims {
        plugin_id: "example.hello".into(),
        tier: "paid".into(),
        major: 1,
        exp: None,
        meters: None,
    };
    let token = SignedToken {
        signature: hex::encode(key.sign(&serde_json::to_vec(&claims).unwrap()).to_bytes()),
        claims,
    };
    let token_file = home.path().join("purchase.json");
    std::fs::write(&token_file, serde_json::to_vec(&token).unwrap()).unwrap();
    run(&["license", "add", token_file.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("pinned"));
    let keys = home.path().join("licenses/trusted-keys");
    std::fs::create_dir_all(&keys).unwrap();
    std::fs::write(
        keys.join("example.hello.pub"),
        hex::encode(key.verifying_key().to_bytes()),
    )
    .unwrap();
    run(&["license", "add", token_file.to_str().unwrap()])
        .assert()
        .success();
    run(&["license", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"valid\": true"))
        .stdout(predicate::str::contains("signature").not());
    run(&["plugin", "tools"])
        .assert()
        .success()
        .stdout(predicate::str::contains("px_hello_greet"));
    run(&[
        "plugin",
        "call",
        "example.hello",
        "greet",
        "--input",
        r#"{"name":"Licensed"}"#,
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Hello, Licensed!"));
    run(&["license", "remove", "example.hello"])
        .assert()
        .success();
    run(&["plugin", "tools"])
        .assert()
        .success()
        .stdout(predicate::str::contains("px_hello_greet").not());
    run(&["license", "remove", "../unsafe"]).assert().failure();
}
