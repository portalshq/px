//! Verified package downloads and the signed-index format. Hosting is deferred.
use crate::{
    manifest::{PluginManifest, valid_plugin_id},
    registry::Store,
};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
    time::Duration,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Index {
    pub schema_version: u32,
    pub plugins: Vec<IndexEntry>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexEntry {
    pub id: String,
    pub version: String,
    pub url: String,
    pub blake3: String,
}

/// The signature covers the exact UTF-8 index bytes, not a reserialized object.
/// Keys are supplied from a pin outside the downloaded index.
pub fn verify_index(bytes: &[u8], signature: &str, key: &VerifyingKey) -> Result<Index, String> {
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("index exceeds size limit".into());
    }
    let signature = Signature::from_slice(
        &hex::decode(signature).map_err(|_| "invalid index signature encoding")?,
    )
    .map_err(|_| "invalid index signature")?;
    key.verify_strict(bytes, &signature)
        .map_err(|_| "invalid index signature")?;
    let index: Index = serde_json::from_slice(bytes).map_err(|e| format!("invalid index: {e}"))?;
    if index.schema_version != 1 {
        return Err("unsupported index schema".into());
    }
    let mut seen = BTreeSet::new();
    for entry in &index.plugins {
        if !valid_plugin_id(&entry.id)
            || semver::Version::parse(&entry.version).is_err()
            || !seen.insert((&entry.id, &entry.version))
        {
            return Err("invalid or duplicate index entry".into());
        }
        validate_hash(&entry.blake3)?;
        validate_url(&entry.url)?;
    }
    Ok(index)
}
fn validate_hash(hash: &str) -> Result<&str, String> {
    let value = hash
        .strip_prefix("blake3:")
        .ok_or("a blake3: checksum is required")?;
    if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("invalid BLAKE3 checksum".into());
    }
    Ok(value)
}
fn validate_url(value: &str) -> Result<reqwest::Url, String> {
    let url = reqwest::Url::parse(value).map_err(|_| "invalid package URL")?;
    if !matches!(url.scheme(), "https" | "http")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("package URL must use HTTP(S) without inline credentials".into());
    }
    Ok(url)
}
pub async fn install_url(
    store: &Store,
    url: &str,
    expected_hash: &str,
) -> Result<PluginManifest, String> {
    let expected = validate_hash(expected_hash)?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;
    let mut response = client
        .get(validate_url(url)?)
        .send()
        .await
        .map_err(|e| e.without_url().to_string())?
        .error_for_status()
        .map_err(|e| e.without_url().to_string())?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| e.without_url().to_string())?
    {
        if bytes.len().saturating_add(chunk.len()) > 128 * 1024 * 1024 {
            return Err("plugin archive exceeds download limit".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    if !blake3::hash(&bytes)
        .to_hex()
        .as_str()
        .eq_ignore_ascii_case(expected)
    {
        return Err("plugin archive BLAKE3 mismatch".into());
    }
    let directory = tempfile::tempdir().map_err(|e| e.to_string())?;
    extract_archive(&bytes, directory.path())?;
    store
        .install_directory(directory.path())
        .map_err(|e| e.to_string())
}

fn extract_archive(bytes: &[u8], directory: &Path) -> Result<(), String> {
    let decoder = flate2::read::GzDecoder::new(bytes);
    let mut archive = tar::Archive::new(decoder);
    let mut total = 0u64;
    let mut paths = BTreeSet::new();
    for (index, entry) in archive.entries().map_err(|e| e.to_string())?.enumerate() {
        if index >= 4096 {
            return Err("plugin archive has too many entries".into());
        }
        let mut entry = entry.map_err(|e| e.to_string())?;
        let kind = entry.header().entry_type();
        if !kind.is_file() && !kind.is_dir() {
            return Err("plugin archive contains a link or special file".into());
        }
        total = total
            .checked_add(entry.size())
            .ok_or("archive size overflow")?;
        if total > 512 * 1024 * 1024 {
            return Err("plugin archive exceeds extracted size limit".into());
        }
        let path = entry.path().map_err(|e| e.to_string())?;
        let mut normalized = PathBuf::new();
        for component in path.components() {
            match component {
                Component::Normal(part) => normalized.push(part),
                Component::CurDir => {}
                _ => return Err("unsafe path in plugin archive".into()),
            }
        }
        if normalized.as_os_str().is_empty() {
            if kind.is_dir() {
                continue;
            }
            return Err("empty archive path".into());
        }
        if !paths.insert(normalized.clone()) {
            return Err("duplicate path in plugin archive".into());
        }
        let mode = entry.header().mode().map_err(|e| e.to_string())?;
        if mode & 0o6000 != 0 {
            return Err("privileged mode in plugin archive".into());
        }
        let destination = directory.join(normalized);
        if kind.is_dir() {
            std::fs::create_dir_all(destination).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&destination)
                .map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut file).map_err(|e| e.to_string())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(
                    destination,
                    std::fs::Permissions::from_mode(mode & 0o777),
                )
                .map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    #[test]
    fn signed_index_requires_the_pinned_key_and_exact_bytes() {
        let index = Index {
            schema_version: 1,
            plugins: vec![IndexEntry {
                id: "example.hello".into(),
                version: "0.1.0".into(),
                url: "https://example.org/plugin.tar.gz".into(),
                blake3: format!("blake3:{}", "a".repeat(64)),
            }],
        };
        let bytes = serde_json::to_vec(&index).unwrap();
        let key = SigningKey::from_bytes(&[4; 32]);
        let signature = hex::encode(key.sign(&bytes).to_bytes());
        assert_eq!(
            verify_index(&bytes, &signature, &key.verifying_key())
                .unwrap()
                .plugins
                .len(),
            1
        );
        assert!(
            verify_index(
                &bytes,
                &signature,
                &SigningKey::from_bytes(&[5; 32]).verifying_key()
            )
            .is_err()
        );
        let mut modified = bytes.clone();
        modified.push(b' ');
        assert!(verify_index(&modified, &signature, &key.verifying_key()).is_err());
    }
    #[test]
    fn archives_reject_links_and_preserve_regular_files() {
        fn archive(link: bool) -> Vec<u8> {
            let mut output = Vec::new();
            {
                let gzip =
                    flate2::write::GzEncoder::new(&mut output, flate2::Compression::default());
                let mut builder = tar::Builder::new(gzip);
                let mut header = tar::Header::new_gnu();
                header.set_mode(0o755);
                header.set_size(if link { 0 } else { 4 });
                if link {
                    header.set_entry_type(tar::EntryType::Symlink);
                    header.set_link_name("/tmp/elsewhere").unwrap();
                }
                header.set_cksum();
                builder
                    .append_data(
                        &mut header,
                        "bin/tool",
                        if link { &b""[..] } else { &b"data"[..] },
                    )
                    .unwrap();
                builder.into_inner().unwrap().finish().unwrap();
            }
            output
        }
        let directory = tempfile::tempdir().unwrap();
        extract_archive(&archive(false), directory.path()).unwrap();
        assert_eq!(
            std::fs::read(directory.path().join("bin/tool")).unwrap(),
            b"data"
        );
        assert!(extract_archive(&archive(true), directory.path()).is_err());
        assert!(validate_hash("sha256:bad").is_err());
        assert!(validate_url("file:///tmp/plugin").is_err());
    }
}
