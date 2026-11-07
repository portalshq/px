//! Offline storage of tokens verified against administratively pinned keys.
use crate::{
    entitlement::{Claims, EntitlementVerifier, SignedToken, TokenVerifier},
    manifest::valid_plugin_id,
};
use ed25519_dalek::VerifyingKey;
use serde::Serialize;
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub struct LicenseStore {
    root: PathBuf,
}
#[derive(Debug, Serialize)]
pub struct Status {
    pub plugin_id: String,
    pub major: Option<u64>,
    pub tier: Option<String>,
    pub valid: bool,
    pub reason: Option<String>,
}
impl LicenseStore {
    pub fn new(px_home: &Path) -> Self {
        Self {
            root: px_home.join("licenses"),
        }
    }
    fn token_path(&self, id: &str) -> Result<PathBuf, String> {
        if !valid_plugin_id(id) {
            return Err("invalid plugin ID".into());
        }
        Ok(self.root.join(format!("{id}.json")))
    }
    fn key(&self, id: &str) -> Result<VerifyingKey, String> {
        self.token_path(id)?;
        let value = fs::read_to_string(self.root.join("trusted-keys").join(format!("{id}.pub")))
            .map_err(|_| "no pinned entitlement key for plugin")?;
        let bytes: [u8; 32] = hex::decode(value.trim())
            .map_err(|_| "invalid pinned key encoding")?
            .try_into()
            .map_err(|_| "invalid pinned key length")?;
        VerifyingKey::from_bytes(&bytes).map_err(|_| "invalid pinned Ed25519 key".into())
    }
    fn read_token(path: &Path) -> Result<SignedToken, String> {
        let mut bytes = Vec::new();
        fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take(65537)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 65536 {
            return Err("token exceeds 64 KiB".into());
        }
        serde_json::from_slice(&bytes).map_err(|e| format!("invalid token: {e}"))
    }
    pub fn verify(&self, id: &str, major: u64, now: i64) -> Result<Claims, String> {
        let token = Self::read_token(&self.token_path(id)?)?;
        TokenVerifier {
            token: &token,
            key: &self.key(id)?,
        }
        .verify(id, major, now)?;
        Ok(token.claims)
    }
    pub fn add(&self, path: &Path, now: i64) -> Result<Claims, String> {
        let token = Self::read_token(path)?;
        let destination = self.token_path(&token.claims.plugin_id)?;
        TokenVerifier {
            token: &token,
            key: &self.key(&token.claims.plugin_id)?,
        }
        .verify(&token.claims.plugin_id, token.claims.major, now)?;
        fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        let mut file = tempfile::NamedTempFile::new_in(&self.root).map_err(|e| e.to_string())?;
        file.write_all(&serde_json::to_vec(&token).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        file.flush().map_err(|e| e.to_string())?;
        file.persist(destination).map_err(|e| e.to_string())?;
        Ok(token.claims)
    }
    pub fn remove(&self, id: &str) -> Result<(), String> {
        fs::remove_file(self.token_path(id)?).map_err(|e| e.to_string())
    }
    pub fn status(&self, now: i64) -> Result<Vec<Status>, String> {
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(e.to_string()),
        };
        let mut result = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name();
            let Some(id) = name.to_str().and_then(|s| s.strip_suffix(".json")) else {
                continue;
            };
            if !valid_plugin_id(id) {
                continue;
            }
            let token = Self::read_token(&entry.path());
            let (major, tier, validation) = match token {
                Ok(token) => (
                    Some(token.claims.major),
                    Some(token.claims.tier.clone()),
                    self.verify(id, token.claims.major, now).map(|_| ()),
                ),
                Err(error) => (None, None, Err(error)),
            };
            result.push(Status {
                plugin_id: id.into(),
                major,
                tier,
                valid: validation.is_ok(),
                reason: validation.err(),
            });
        }
        result.sort_by(|a, b| a.plugin_id.cmp(&b.plugin_id));
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    #[test]
    fn license_files_require_pinned_keys_and_valid_claims() {
        let home = tempfile::tempdir().unwrap();
        let store = LicenseStore::new(home.path());
        let key = SigningKey::from_bytes(&[11; 32]);
        let claims = Claims {
            plugin_id: "example.paid".into(),
            tier: "pro".into(),
            major: 1,
            exp: Some(100),
            meters: None,
        };
        let token = SignedToken {
            signature: hex::encode(key.sign(&serde_json::to_vec(&claims).unwrap()).to_bytes()),
            claims,
        };
        let source = home.path().join("token.json");
        fs::write(&source, serde_json::to_vec(&token).unwrap()).unwrap();
        assert!(store.status(0).unwrap().is_empty());
        assert!(store.add(&source, 0).is_err());
        fs::create_dir_all(home.path().join("licenses/trusted-keys")).unwrap();
        fs::write(
            home.path().join("licenses/trusted-keys/example.paid.pub"),
            hex::encode(key.verifying_key().to_bytes()),
        )
        .unwrap();
        store.add(&source, 0).unwrap();
        assert!(store.verify("example.paid", 1, 0).is_ok());
        assert!(store.verify("example.paid", 2, 0).is_err());
        assert!(store.status(0).unwrap()[0].valid);
        assert!(!store.status(100).unwrap()[0].valid);
        assert!(store.add(&source, 100).is_err());
        assert!(store.remove("../invalid").is_err());
        store.remove("example.paid").unwrap();
        assert!(store.status(0).unwrap().is_empty());
    }
}
