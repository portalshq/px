#![cfg(unix)]
use assert_cmd::Command;
use std::{fs, os::unix::fs::PermissionsExt};
#[test]
fn install_uv_validates_an_explicit_runtime_without_downloading() {
    let home = tempfile::tempdir().unwrap();
    let binary = home.path().join("uv-fixture");
    fs::write(&binary, "#!/bin/sh\necho 'uv 0.10.12 (fixture)'\n").unwrap();
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
    let run = || {
        let mut command = Command::new(assert_cmd::cargo::cargo_bin!("px"));
        command
            .arg("--base-dir")
            .arg(home.path())
            .args(["install", "uv"])
            .env("PX_UV_PATH", &binary);
        command
    };
    run().assert().success();
    fs::write(&binary, "#!/bin/sh\necho 'uv 0.1.0'\n").unwrap();
    run().assert().failure();
}
