//! Verify an optimizer release with a separately trusted local pin.
use px_plugin::release::{ReleasePin, verify_artifact, verify_manifest};
use std::{env, fs, process};

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 5 {
        return Err(
            "usage: px-verify-optimizer-release PIN MANIFEST SIGNATURE TARGET ARCHIVE".into(),
        );
    }
    let pin: ReleasePin = serde_json::from_slice(&fs::read(&args[0])?)?;
    let manifest = fs::read(&args[1])?;
    let signature = fs::read_to_string(&args[2])?;
    let target = args[3].to_str().ok_or("invalid target encoding")?;
    let checked = verify_manifest(&manifest, &signature, &pin, env!("CARGO_PKG_VERSION"))?;
    if fs::metadata(&args[4])?.len() > 1024 * 1024 * 1024 {
        return Err("release archive exceeds size limit".into());
    }
    verify_artifact(&checked, target, &fs::read(&args[4])?)?;
    println!(
        "verified {} {} for {}",
        checked.plugin_id, checked.version, target
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("release verification failed: {error}");
        process::exit(1);
    }
}
