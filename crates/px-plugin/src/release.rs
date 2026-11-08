//! Verification primitives for a pinned, signed optimizer release.
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const TARGETS: [&str; 5] = [
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-pc-windows-msvc",
];

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub size: u64,
    pub blake3: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseManifest {
    pub schema_version: u32,
    pub plugin_id: String,
    pub version: String,
    pub release_id: String,
    pub px_compatibility: String,
    pub px_commit: String,
    pub optimizer_commit: String,
    pub protocol_version: u32,
    pub artifact_id: String,
    pub artifact_blake3: String,
    pub targets: BTreeMap<String, Artifact>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleasePin {
    pub plugin_id: String,
    pub version: String,
    pub release_id: String,
    pub manifest_blake3: String,
    pub public_key: String,
}

fn digest_valid(value: &str) -> bool {
    value.strip_prefix("blake3:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    })
}

/// Verify the exact signed bytes against a separately trusted, committed pin.
pub fn verify_manifest(
    bytes: &[u8],
    signature_hex: &str,
    pin: &ReleasePin,
    px_version: &str,
) -> Result<ReleaseManifest, String> {
    if bytes.len() > 1024 * 1024
        || !digest_valid(&pin.manifest_blake3)
        || format!("blake3:{}", blake3::hash(bytes).to_hex()) != pin.manifest_blake3
        || pin.public_key.len() != 64
        || signature_hex.trim().len() != 128
    {
        return Err("release manifest does not match its pin".into());
    }
    let key: [u8; 32] = hex::decode(&pin.public_key)
        .map_err(|_| "invalid release public key")?
        .try_into()
        .map_err(|_| "invalid release public key length")?;
    let key = VerifyingKey::from_bytes(&key).map_err(|_| "invalid release public key")?;
    let signature = Signature::from_slice(
        &hex::decode(signature_hex.trim()).map_err(|_| "invalid signature encoding")?,
    )
    .map_err(|_| "invalid signature length")?;
    key.verify_strict(bytes, &signature)
        .map_err(|_| "invalid release signature")?;
    let manifest: ReleaseManifest =
        serde_json::from_slice(bytes).map_err(|_| "invalid release manifest")?;
    let version = semver::Version::parse(px_version).map_err(|_| "invalid PX version")?;
    let req = semver::VersionReq::parse(&manifest.px_compatibility)
        .map_err(|_| "invalid PX compatibility range")?;
    if manifest.schema_version != 1
        || manifest.plugin_id != "px.optimizer"
        || manifest.plugin_id != pin.plugin_id
        || manifest.version != pin.version
        || manifest.release_id != pin.release_id
        || manifest.release_id != format!("optimizer-v{}", manifest.version)
        || semver::Version::parse(&manifest.version).is_err()
        || manifest.protocol_version != 1
        || !req.matches(&version)
        || !digest_valid(&manifest.artifact_blake3)
        || manifest.artifact_id.trim().is_empty()
        || [&manifest.px_commit, &manifest.optimizer_commit]
            .iter()
            .any(|s| s.len() != 40 || !s.bytes().all(|b| b.is_ascii_hexdigit()))
        || manifest.targets.len() != TARGETS.len()
        || TARGETS
            .iter()
            .any(|target| !manifest.targets.contains_key(*target))
        || manifest
            .targets
            .values()
            .any(|a| a.size == 0 || !digest_valid(&a.blake3))
    {
        return Err("release manifest has invalid or incompatible claims".into());
    }
    Ok(manifest)
}

pub fn verify_artifact(
    manifest: &ReleaseManifest,
    target: &str,
    bytes: &[u8],
) -> Result<(), String> {
    let expected = manifest
        .targets
        .get(target)
        .ok_or("release target not found")?;
    if bytes.len() as u64 != expected.size
        || format!("blake3:{}", blake3::hash(bytes).to_hex()) != expected.blake3
    {
        return Err("release artifact does not match signed manifest".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    #[test]
    fn verifies_pinned_signature_targets_and_artifact() {
        let key = SigningKey::from_bytes(&[7; 32]);
        let data = b"fixture archive";
        let manifest = ReleaseManifest {
            schema_version: 1,
            plugin_id: "px.optimizer".into(),
            version: "0.1.0".into(),
            release_id: "optimizer-v0.1.0".into(),
            px_compatibility: ">=0.10, <0.11".into(),
            px_commit: "a".repeat(40),
            optimizer_commit: "b".repeat(40),
            protocol_version: 1,
            artifact_id: "baseline".into(),
            artifact_blake3: format!("blake3:{}", blake3::hash(b"state").to_hex()),
            targets: TARGETS
                .iter()
                .map(|t| {
                    (
                        (*t).into(),
                        Artifact {
                            size: data.len() as u64,
                            blake3: format!("blake3:{}", blake3::hash(data).to_hex()),
                        },
                    )
                })
                .collect(),
        };
        let bytes = serde_json::to_vec(&manifest).unwrap();
        let pin = ReleasePin {
            plugin_id: "px.optimizer".into(),
            version: "0.1.0".into(),
            release_id: "optimizer-v0.1.0".into(),
            manifest_blake3: format!("blake3:{}", blake3::hash(&bytes).to_hex()),
            public_key: hex::encode(key.verifying_key().to_bytes()),
        };
        let signature = hex::encode(key.sign(&bytes).to_bytes());
        let checked = verify_manifest(&bytes, &signature, &pin, "0.10.0").unwrap();
        assert!(verify_artifact(&checked, TARGETS[0], data).is_ok());
        assert!(verify_artifact(&checked, TARGETS[0], b"tampered archive").is_err());
        assert!(verify_manifest(&bytes, &signature, &pin, "0.11.0").is_err());
        let other = SigningKey::from_bytes(&[8; 32]);
        assert!(
            verify_manifest(
                &bytes,
                &hex::encode(other.sign(&bytes).to_bytes()),
                &pin,
                "0.10.0"
            )
            .is_err()
        );
        let mut tampered = bytes.clone();
        tampered.push(b' ');
        assert!(verify_manifest(&tampered, &signature, &pin, "0.10.0").is_err());
        let mut incomplete: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        incomplete["targets"]
            .as_object_mut()
            .unwrap()
            .remove(TARGETS[0]);
        let incomplete = serde_json::to_vec(&incomplete).unwrap();
        let incomplete_pin = ReleasePin {
            manifest_blake3: format!("blake3:{}", blake3::hash(&incomplete).to_hex()),
            ..pin
        };
        assert!(
            verify_manifest(
                &incomplete,
                &hex::encode(key.sign(&incomplete).to_bytes()),
                &incomplete_pin,
                "0.10.0"
            )
            .is_err()
        );
        for (field, value) in [
            ("plugin_id", serde_json::json!("example.wrong")),
            ("version", serde_json::json!("9.0.0")),
            ("release_id", serde_json::json!("optimizer-v9.0.0")),
            ("protocol_version", serde_json::json!(2)),
        ] {
            let mut altered: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            altered[field] = value;
            let altered = serde_json::to_vec(&altered).unwrap();
            let altered_pin = ReleasePin {
                plugin_id: incomplete_pin.plugin_id.clone(),
                version: incomplete_pin.version.clone(),
                release_id: incomplete_pin.release_id.clone(),
                manifest_blake3: format!("blake3:{}", blake3::hash(&altered).to_hex()),
                public_key: incomplete_pin.public_key.clone(),
            };
            assert!(
                verify_manifest(
                    &altered,
                    &hex::encode(key.sign(&altered).to_bytes()),
                    &altered_pin,
                    "0.10.0"
                )
                .is_err()
            );
        }
    }
}
