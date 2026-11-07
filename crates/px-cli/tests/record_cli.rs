//! Exercise the real core command boundaries with a deterministic Lore process.
#![cfg(unix)]
use assert_cmd::Command;
use px_core::{ContentHash, Manifest};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

struct Checkout {
    home: tempfile::TempDir,
    lore: PathBuf,
    log: PathBuf,
}
impl Checkout {
    fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let repo = home.path().join("demo");
        fs::create_dir_all(repo.join(".lore")).unwrap();
        fs::create_dir(repo.join("character")).unwrap();
        Manifest::new("demo", "world".into(), "demo", "Demo")
            .to_file(&repo.join("repository.yaml"))
            .unwrap();
        Manifest::new("demo", "character".into(), "atlas", "Atlas")
            .to_file(&repo.join("character/atlas.yaml"))
            .unwrap();
        fs::write(
            repo.join(".lore/config.toml"),
            "remote_url = 'lore://127.0.0.1:1'\n[file]\ndirect_io = false\n",
        )
        .unwrap();
        let lore = home.path().join("lore-fixture");
        let log = home.path().join("calls");
        let script=r#"#!/bin/sh
if [ "$1" = --version ]; then printf '%s\n' '__VERSION__'; exit 0; fi
printf '%s %s\n' "$1" "$2" >> "$PX_TEST_CALLS"
case "$1 $2" in
  'branch list') printf '%s\n' '{"tagName":"branchListEntry","data":{"name":"story","location":"local","isCurrent":true}}';;
  'revision commit') printf '%s\n' 'Signature : bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';;
  'history 1') printf '%s\n' 'Signature : aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';;
esac
"#.replace("__VERSION__",px_core::PINNED_LORE_VERSION);
        fs::write(&lore, script).unwrap();
        fs::set_permissions(&lore, fs::Permissions::from_mode(0o755)).unwrap();
        Self { home, lore, log }
    }
    fn command(&self) -> Command {
        let mut command = Command::new(assert_cmd::cargo::cargo_bin!("px"));
        command
            .arg("--base-dir")
            .arg(self.home.path())
            .env("PXLORE_CLI", &self.lore)
            .env("PX_TEST_CALLS", &self.log);
        command
    }
    fn manifest(&self) -> Manifest {
        Manifest::from_file(&self.home.path().join("demo/character/atlas.yaml")).unwrap()
    }
}
#[test]
fn record_persists_exact_prompt_hashes_and_metadata_with_one_commit_and_push() {
    let checkout = Checkout::new();
    let output = checkout.home.path().join("generated.png");
    fs::write(&output, b"generated pixels").unwrap();
    let prompt = "Atlas, exact `prompt` with $tokens\nand a second line.";
    let result = checkout
        .command()
        .args([
            "record",
            "px://demo/character/atlas",
            "--branch",
            "story",
            "--output",
        ])
        .arg(format!("portrait={}", output.display()))
        .args([
            "--prompt",
            prompt,
            "--provider",
            "test",
            "--model",
            "model-returned",
            "--parameters",
            r#"{"seed":4}"#,
            "--provenance-json",
            r#"{"adapter_id":"generic","execution_mode":"host"}"#,
        ])
        .assert()
        .success();
    let record: Value = serde_json::from_slice(&result.get_output().stdout).unwrap();
    assert_eq!(
        record["outputs"][0]["hash"],
        ContentHash::from_bytes(b"generated pixels").as_str()
    );
    let manifest = checkout.manifest();
    assert_eq!(
        manifest.representations["portrait_prompt"].hash,
        ContentHash::from_str_content(prompt).as_str()
    );
    assert_eq!(
        fs::read_to_string(
            checkout
                .home
                .path()
                .join("demo/character/atlas/portrait_prompt.txt")
        )
        .unwrap(),
        prompt
    );
    assert_eq!(
        manifest.metadata["generation"]["portrait"]["prompt_text"].as_str(),
        Some(prompt)
    );
    assert_eq!(
        manifest.metadata["generation"]["portrait"]["model"].as_str(),
        Some("model-returned")
    );
    let calls = fs::read_to_string(&checkout.log).unwrap();
    assert_eq!(
        calls
            .lines()
            .filter(|line| *line == "revision commit")
            .count(),
        1
    );
    assert_eq!(
        calls.lines().filter(|line| *line == "branch push").count(),
        1
    );
    let created = manifest.metadata["generation"]["portrait"]["date_created"].clone();
    let first_feedback = json!({"uri":"px://demo/character/atlas","branch":"story","key":"portrait","accepted":true,"rating":5});
    checkout
        .command()
        .args(["record-feedback", "--input", &first_feedback.to_string()])
        .assert()
        .success();
    fs::write(&output, b"updated pixels").unwrap();
    checkout
        .command()
        .args([
            "record",
            "px://demo/character/atlas",
            "--branch",
            "story",
            "--output",
        ])
        .arg(format!("portrait={}", output.display()))
        .args(["--prompt", prompt, "--replace"])
        .assert()
        .success();
    assert_eq!(
        checkout.manifest().metadata["generation"]["portrait"]["date_created"],
        created
    );
    assert_eq!(
        checkout.manifest().metadata["feedback"]["portrait"]["stale"].as_bool(),
        Some(true)
    );
    let feedback = json!({"uri":"px://demo/character/atlas","branch":"story","key":"portrait","accepted":true,"rating":5});
    checkout
        .command()
        .args(["record-feedback", "--input", &feedback.to_string()])
        .assert()
        .success();
    assert_eq!(
        checkout.manifest().metadata["feedback"]["portrait"]["export_training"].as_bool(),
        Some(false)
    );
}
#[test]
fn record_rejects_wrong_branch_and_invalid_parameters_without_staging_outputs() {
    let checkout = Checkout::new();
    let output = checkout.home.path().join("generated.png");
    fs::write(&output, b"pixels").unwrap();
    for (branch, parameters) in [("wrong", "{}"), ("story", "[]")] {
        checkout
            .command()
            .args([
                "record",
                "px://demo/character/atlas",
                "--branch",
                branch,
                "--output",
            ])
            .arg(format!("portrait={}", output.display()))
            .args(["--prompt", "exact", "--parameters", parameters])
            .assert()
            .failure();
    }
    assert!(checkout.manifest().representations.is_empty());
    let calls = fs::read_to_string(&checkout.log).unwrap_or_default();
    assert!(!calls.contains("revision commit"));
}
