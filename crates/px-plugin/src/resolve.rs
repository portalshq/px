//! Compute the effective installed plugin set without executing packages.
use crate::{
    manifest::{Commerce, PluginManifest},
    registry::Store,
};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Exclusion {
    Disabled,
    Unlicensed,
    Incompatible,
    MissingEntrypoint,
    InvalidManifest,
    Unpinned,
}
#[derive(Debug)]
pub struct EffectivePlugin {
    pub manifest: PluginManifest,
    pub entrypoint: PathBuf,
    pub entitlement: Option<crate::entitlement::Claims>,
}
#[derive(Debug)]
pub struct Resolution {
    pub enabled: Vec<EffectivePlugin>,
    pub excluded: BTreeMap<String, Exclusion>,
}

/// `pins`, when supplied, is an allow-list of plugin IDs and exact versions.
/// Paid plugins require a valid offline token against a locally pinned issuer key.
pub fn effective(
    store: &Store,
    px_version: &str,
    platform: &str,
    pins: Option<&BTreeMap<String, String>>,
) -> Result<Resolution, String> {
    let registry = store.load().map_err(|e| e.to_string())?;
    let version = semver::Version::parse(px_version).map_err(|e| e.to_string())?;
    let mut result = Resolution {
        enabled: vec![],
        excluded: BTreeMap::new(),
    };
    for (id, installed) in registry.installed {
        let excluded = if !installed.enabled {
            Some(Exclusion::Disabled)
        } else if pins.is_some_and(|pins| pins.get(&id) != Some(&installed.version)) {
            Some(Exclusion::Unpinned)
        } else {
            None
        };
        if let Some(reason) = excluded {
            result.excluded.insert(id, reason);
            continue;
        }
        let manifest = match store.manifest(&id) {
            Ok(m) => m,
            Err(_) => {
                result.excluded.insert(id, Exclusion::InvalidManifest);
                continue;
            }
        };
        if !semver::VersionReq::parse(&manifest.px).is_ok_and(|req| req.matches(&version)) {
            result.excluded.insert(id, Exclusion::Incompatible);
            continue;
        }
        let major = match &manifest.commerce {
            Commerce::Free => None,
            Commerce::OneTime { major, .. } => Some(*major),
            Commerce::Usage { .. } => Some(
                semver::Version::parse(&manifest.version)
                    .map_err(|e| e.to_string())?
                    .major,
            ),
        };
        let entitlement = if let Some(major) = major {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_secs();
            match crate::license::LicenseStore::new(store.px_home()).verify(
                &id,
                major,
                i64::try_from(now).map_err(|e| e.to_string())?,
            ) {
                Ok(claims) => Some(claims),
                Err(_) => {
                    result.excluded.insert(id, Exclusion::Unlicensed);
                    continue;
                }
            }
        } else {
            None
        };
        let Some(relative) = manifest.entrypoint.get(platform) else {
            result.excluded.insert(id, Exclusion::MissingEntrypoint);
            continue;
        };
        let root = store
            .plugin_dir(&id, &installed.version)
            .map_err(|e| e.to_string())?;
        let path = root.join(relative);
        let safe = root
            .canonicalize()
            .ok()
            .zip(path.canonicalize().ok())
            .is_some_and(|(root, path)| path.starts_with(root) && path.is_file());
        if !safe {
            result.excluded.insert(id, Exclusion::MissingEntrypoint);
            continue;
        }
        result.enabled.push(EffectivePlugin {
            manifest,
            entrypoint: path,
            entitlement,
        });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resolution_respects_enabled_pins_version_and_platform() {
        let home = tempfile::tempdir().unwrap();
        let store = Store::new(home.path());
        let source =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/hello");
        store.install_directory(&source).unwrap();
        let platform = "aarch64-apple-darwin";
        assert_eq!(
            effective(&store, "0.9.8", platform, None).unwrap().excluded["example.hello"],
            Exclusion::Disabled
        );
        store.set_enabled("example.hello", true).unwrap();
        assert_eq!(
            effective(&store, "0.9.8", platform, None)
                .unwrap()
                .enabled
                .len(),
            1
        );
        assert_eq!(
            effective(&store, "0.9.8", platform, Some(&BTreeMap::new()))
                .unwrap()
                .excluded["example.hello"],
            Exclusion::Unpinned
        );
        assert_eq!(
            effective(&store, "1.0.0", platform, None).unwrap().excluded["example.hello"],
            Exclusion::Incompatible
        );
        assert_eq!(
            effective(&store, "0.9.8", "unknown", None)
                .unwrap()
                .excluded["example.hello"],
            Exclusion::MissingEntrypoint
        );
    }
}
