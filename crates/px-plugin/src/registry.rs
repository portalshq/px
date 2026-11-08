//! Local registry. Installing a plugin does not execute it or enable it.
use crate::manifest::PluginManifest;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),
    #[error(transparent)]
    Manifest(#[from] crate::manifest::ManifestError),
    #[error("{0}")]
    Invalid(String),
}
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Registry {
    #[serde(default)]
    pub installed: BTreeMap<String, InstalledPlugin>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPlugin {
    pub version: String,
    pub namespace: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PendingUpgrade {
    pub id: String,
    pub previous: InstalledPlugin,
    pub target_version: String,
}

pub struct Store {
    root: PathBuf,
}
impl Store {
    pub fn new(px_home: &Path) -> Self {
        Self {
            root: px_home.join("plugins"),
        }
    }
    pub fn px_home(&self) -> &Path {
        self.root.parent().expect("plugin registry has a parent")
    }
    /// Serialize CLI registry changes and recovery across processes. The OS
    /// releases this advisory lock automatically when the process exits.
    pub fn acquire_lock(&self) -> Result<fs::File, RegistryError> {
        fs::create_dir_all(&self.root)?;
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(self.root.join("registry.lock"))?;
        file.lock()?;
        Ok(file)
    }
    pub fn load(&self) -> Result<Registry, RegistryError> {
        match fs::read_to_string(self.root.join("registry.yaml")) {
            Ok(s) => Ok(serde_yaml::from_str(&s)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Registry::default()),
            Err(e) => Err(e.into()),
        }
    }
    pub fn plugin_dir(&self, id: &str, version: &str) -> Result<PathBuf, RegistryError> {
        if id.split('.').count() < 2
            || id.split('.').any(|part| {
                part.is_empty()
                    || !part
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            })
            || semver::Version::parse(version).is_err()
        {
            return Err(RegistryError::Invalid("invalid registry identity".into()));
        }
        Ok(self.root.join(id).join(version))
    }
    pub fn manifest(&self, id: &str) -> Result<PluginManifest, RegistryError> {
        let registry = self.load()?;
        let installed = registry
            .installed
            .get(id)
            .ok_or_else(|| RegistryError::Invalid(format!("plugin '{id}' is not installed")))?;
        let manifest: PluginManifest = serde_yaml::from_str(&fs::read_to_string(
            self.plugin_dir(id, &installed.version)?.join("plugin.yaml"),
        )?)?;
        if manifest.id != id
            || manifest.version != installed.version
            || manifest.namespace != installed.namespace
        {
            return Err(RegistryError::Invalid(
                "installed manifest does not match registry".into(),
            ));
        }
        manifest.validate(&BTreeSet::new())?;
        Ok(manifest)
    }
    fn save(&self, registry: &Registry) -> Result<(), RegistryError> {
        fs::create_dir_all(&self.root)?;
        let mut temp = tempfile::NamedTempFile::new_in(&self.root)?;
        temp.write_all(serde_yaml::to_string(registry)?.as_bytes())?;
        temp.persist(self.root.join("registry.yaml"))
            .map_err(|e| e.error)?;
        Ok(())
    }
    pub fn install_directory(&self, source: &Path) -> Result<PluginManifest, RegistryError> {
        let manifest: PluginManifest =
            serde_yaml::from_str(&fs::read_to_string(source.join("plugin.yaml"))?)?;
        let mut registry = self.load()?;
        let occupied: BTreeSet<_> = registry
            .installed
            .values()
            .map(|item| item.namespace.clone())
            .collect();
        manifest.validate(&occupied)?;
        if registry.installed.contains_key(&manifest.id) {
            return Err(RegistryError::Invalid(format!(
                "plugin '{}' is already installed",
                manifest.id
            )));
        }
        fs::create_dir_all(&self.root)?;
        let source_path = fs::canonicalize(source)?;
        let root_path = fs::canonicalize(&self.root)?;
        if root_path.starts_with(&source_path) {
            return Err(RegistryError::Invalid(
                "plugin source cannot contain the plugin registry".into(),
            ));
        }
        let temp = tempfile::Builder::new()
            .prefix(".install-")
            .tempdir_in(&self.root)?;
        copy_tree(
            source,
            temp.path(),
            matches!(manifest.kind, crate::manifest::PluginKind::UvPython),
            true,
        )?;
        let parent = self.root.join(&manifest.id);
        fs::create_dir_all(&parent)?;
        let destination = parent.join(&manifest.version);
        if destination.exists() {
            return Err(RegistryError::Invalid(
                "plugin version already exists on disk".into(),
            ));
        }
        fs::rename(temp.path(), &destination)?;
        registry.installed.insert(
            manifest.id.clone(),
            InstalledPlugin {
                version: manifest.version.clone(),
                namespace: manifest.namespace.clone(),
                enabled: false,
            },
        );
        if let Err(error) = self.save(&registry) {
            let _ = fs::remove_dir_all(&destination);
            return Err(error);
        }
        Ok(manifest)
    }

    /// Stage and validate a new version before switching the registry. The
    /// previous version remains on disk so a caller can restore it if skill
    /// installation fails. This method does not update harness skills.
    pub fn upgrade_directory(
        &self,
        source: &Path,
        px_version: &str,
        platform: &str,
    ) -> Result<(PluginManifest, InstalledPlugin), RegistryError> {
        let manifest: PluginManifest =
            serde_yaml::from_str(&fs::read_to_string(source.join("plugin.yaml"))?)?;
        let mut registry = self.load()?;
        let previous = registry
            .installed
            .get(&manifest.id)
            .cloned()
            .ok_or_else(|| {
                RegistryError::Invalid("plugin is not installed; use install instead".into())
            })?;
        let occupied: BTreeSet<_> = registry
            .installed
            .iter()
            .filter(|(id, _)| *id != &manifest.id)
            .map(|(_, item)| item.namespace.clone())
            .collect();
        manifest.validate(&occupied)?;
        if manifest.namespace != previous.namespace {
            return Err(RegistryError::Invalid(
                "upgrade cannot change plugin namespace".into(),
            ));
        }
        let old_version = semver::Version::parse(&previous.version)
            .map_err(|_| RegistryError::Invalid("invalid installed version".into()))?;
        let new_version = semver::Version::parse(&manifest.version)
            .map_err(|_| RegistryError::Invalid("invalid candidate version".into()))?;
        if new_version <= old_version {
            return Err(RegistryError::Invalid(
                "upgrade requires a newer version".into(),
            ));
        }
        let version = semver::Version::parse(px_version)
            .map_err(|_| RegistryError::Invalid("invalid PX version".into()))?;
        if !semver::VersionReq::parse(&manifest.px).is_ok_and(|r| r.matches(&version)) {
            return Err(RegistryError::Invalid(
                "upgrade is incompatible with PX".into(),
            ));
        }
        let relative = manifest.entrypoint.get(platform).ok_or_else(|| {
            RegistryError::Invalid("upgrade has no entrypoint for this platform".into())
        })?;
        fs::create_dir_all(&self.root)?;
        let source_path = fs::canonicalize(source)?;
        let root_path = fs::canonicalize(&self.root)?;
        if root_path.starts_with(&source_path) {
            return Err(RegistryError::Invalid(
                "plugin source cannot contain the plugin registry".into(),
            ));
        }
        let temp = tempfile::Builder::new()
            .prefix(".upgrade-")
            .tempdir_in(&self.root)?;
        copy_tree(
            source,
            temp.path(),
            matches!(manifest.kind, crate::manifest::PluginKind::UvPython),
            true,
        )?;
        let staged_root = temp.path().canonicalize()?;
        let entry = temp
            .path()
            .join(relative)
            .canonicalize()
            .map_err(|_| RegistryError::Invalid("upgrade entrypoint is missing".into()))?;
        if !entry.starts_with(&staged_root) || !entry.is_file() {
            return Err(RegistryError::Invalid(
                "upgrade entrypoint is invalid".into(),
            ));
        }
        for skill in &manifest.skills {
            if !temp.path().join(skill).join("SKILL.md").is_file() {
                return Err(RegistryError::Invalid("upgrade skill is missing".into()));
            }
        }
        let destination = self.plugin_dir(&manifest.id, &manifest.version)?;
        if destination.exists() {
            return Err(RegistryError::Invalid(
                "plugin version already exists on disk".into(),
            ));
        }
        fs::create_dir_all(destination.parent().expect("plugin has a parent"))?;
        let pending = PendingUpgrade {
            id: manifest.id.clone(),
            previous: previous.clone(),
            target_version: manifest.version.clone(),
        };
        let journal_path = self.root.join("upgrade-pending.yaml");
        let mut journal = tempfile::NamedTempFile::new_in(&self.root)?;
        journal.write_all(serde_yaml::to_string(&pending)?.as_bytes())?;
        journal.as_file().sync_all()?;
        journal
            .persist_noclobber(&journal_path)
            .map_err(|error| error.error)?;
        if let Err(error) = fs::rename(temp.path(), &destination) {
            let _ = fs::remove_file(&journal_path);
            return Err(error.into());
        }
        registry.installed.insert(
            manifest.id.clone(),
            InstalledPlugin {
                version: manifest.version.clone(),
                namespace: manifest.namespace.clone(),
                enabled: previous.enabled,
            },
        );
        if let Err(error) = self.save(&registry) {
            let _ = fs::remove_file(&journal_path);
            let _ = fs::remove_dir_all(&destination);
            return Err(error);
        }
        Ok((manifest, previous))
    }

    pub fn pending_upgrade(&self) -> Result<Option<PendingUpgrade>, RegistryError> {
        match fs::read(self.root.join("upgrade-pending.yaml")) {
            Ok(bytes) => Ok(Some(serde_yaml::from_slice(&bytes)?)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    /// Restore the prior registry entry, retaining both package versions until
    /// the caller restores any affected harness skills and clears the journal.
    pub fn rollback_pending_upgrade(&self, pending: &PendingUpgrade) -> Result<(), RegistryError> {
        let mut registry = self.load()?;
        let current = registry
            .installed
            .get(&pending.id)
            .ok_or_else(|| RegistryError::Invalid("upgrade registry entry is missing".into()))?;
        if current.version != pending.target_version && current.version != pending.previous.version
        {
            return Err(RegistryError::Invalid(
                "registry changed during upgrade recovery".into(),
            ));
        }
        self.plugin_dir(&pending.id, &pending.previous.version)?;
        registry
            .installed
            .insert(pending.id.clone(), pending.previous.clone());
        self.save(&registry)
    }

    pub fn clear_upgrade_journal(&self) -> Result<(), RegistryError> {
        match fs::remove_file(self.root.join("upgrade-pending.yaml")) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
    pub fn set_enabled(&self, id: &str, enabled: bool) -> Result<(), RegistryError> {
        let mut registry = self.load()?;
        let item = registry
            .installed
            .get_mut(id)
            .ok_or_else(|| RegistryError::Invalid(format!("plugin '{id}' is not installed")))?;
        item.enabled = enabled;
        self.save(&registry)
    }

    /// Update the repository's exact-version allow-list, preserving unrelated
    /// configuration and the implicit global selection for all other plugins.
    pub fn set_repository_enabled(
        &self,
        repository_dir: &Path,
        id: &str,
        enabled: bool,
    ) -> Result<(), RegistryError> {
        let manifest = self.manifest(id)?;
        let config_dir = repository_dir.join(".px");
        let config_path = config_dir.join("config.yaml");
        let mut config: serde_yaml::Value = match fs::read_to_string(&config_path) {
            Ok(text) => serde_yaml::from_str(&text)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                serde_yaml::Value::Mapping(Default::default())
            }
            Err(error) => return Err(error.into()),
        };
        let mapping = config
            .as_mapping_mut()
            .ok_or_else(|| RegistryError::Invalid("repository config must be a mapping".into()))?;
        let key = serde_yaml::Value::String("plugins".into());
        let mut pins: BTreeMap<String, String> = match mapping.get(&key) {
            Some(value) => serde_yaml::from_value(value.clone())?,
            None => self
                .load()?
                .installed
                .into_iter()
                .filter(|(_, p)| p.enabled)
                .map(|(id, p)| (id, p.version))
                .collect(),
        };
        if enabled {
            pins.insert(id.to_string(), manifest.version);
        } else {
            pins.remove(id);
        }
        mapping.insert(key, serde_yaml::to_value(pins)?);
        fs::create_dir_all(&config_dir)?;
        let mut file = tempfile::NamedTempFile::new_in(&config_dir)?;
        file.write_all(serde_yaml::to_string(&config)?.as_bytes())?;
        file.flush()?;
        file.persist(config_path).map_err(|error| error.error)?;
        Ok(())
    }

    pub fn uninstall(&self, id: &str) -> Result<(), RegistryError> {
        let mut registry = self.load()?;
        let installed = registry
            .installed
            .get(id)
            .ok_or_else(|| RegistryError::Invalid(format!("plugin '{id}' is not installed")))?;
        let source = self.plugin_dir(id, &installed.version)?;
        let metadata = fs::symlink_metadata(&source)?;
        if !metadata.file_type().is_dir() {
            return Err(RegistryError::Invalid(
                "installed plugin is not a directory".into(),
            ));
        }
        let temp = tempfile::Builder::new()
            .prefix(".remove-")
            .tempdir_in(&self.root)?;
        let moved = temp.path().join("plugin");
        fs::rename(&source, &moved)?;
        registry.installed.remove(id);
        if let Err(error) = self.save(&registry) {
            fs::rename(&moved, &source)?;
            return Err(error);
        }
        Ok(())
    }
}
fn copy_tree(source: &Path, target: &Path, python: bool, root: bool) -> Result<(), RegistryError> {
    if !fs::symlink_metadata(source)?.file_type().is_dir() {
        return Err(RegistryError::Invalid(
            "plugin source must be a directory".into(),
        ));
    }
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        if python
            && ((root && entry.file_name() == ".venv")
                || entry.file_name() == "__pycache__"
                || entry.file_name() == ".pytest_cache")
        {
            continue;
        }
        let file_type = entry.file_type()?;
        let dest = target.join(entry.file_name());
        if file_type.is_dir() {
            fs::create_dir(&dest)?;
            copy_tree(&entry.path(), &dest, python, false)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), dest)?;
        } else {
            return Err(RegistryError::Invalid(
                "plugin package may contain only regular files and directories".into(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn upgrade_validates_before_switch_and_preserves_disabled_state() {
        let home = tempfile::tempdir().unwrap();
        let store = Store::new(&home.path().join("px"));
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/hello");
        store.install_directory(&source).unwrap();
        let candidate = home.path().join("candidate");
        fs::create_dir(&candidate).unwrap();
        copy_tree(&source, &candidate, false, true).unwrap();
        let path = candidate.join("plugin.yaml");
        let text = fs::read_to_string(&path)
            .unwrap()
            .replace("version: 0.1.0", "version: 0.2.0");
        fs::write(&path, &text).unwrap();
        assert!(
            store
                .upgrade_directory(&candidate, "0.11.0", "aarch64-apple-darwin")
                .is_err()
        );
        assert_eq!(
            store.load().unwrap().installed["example.hello"].version,
            "0.1.0"
        );
        let (_, previous) = store
            .upgrade_directory(&candidate, "0.10.0", "aarch64-apple-darwin")
            .unwrap();
        assert!(!previous.enabled);
        let installed = store.load().unwrap();
        assert_eq!(installed.installed["example.hello"].version, "0.2.0");
        assert!(!installed.installed["example.hello"].enabled);
        assert!(store.plugin_dir("example.hello", "0.1.0").unwrap().is_dir());
    }
    #[test]
    fn python_plugin_install_excludes_existing_environment() {
        let home = tempfile::tempdir().unwrap();
        let source = home.path().join("source");
        fs::create_dir_all(source.join(".venv")).unwrap();
        fs::write(source.join(".venv/marker"), "local environment").unwrap();
        fs::write(source.join("plugin.yaml"), "id: example.python\nnamespace: python\nversion: 0.1.0\npx: \">=0.9, <0.11\"\nlicense: PolyForm-Shield-1.0.0\nkind: uv-python\nentrypoint: {x: main.py}\ntools: []\ncommerce: {model: free}\n").unwrap();
        fs::write(source.join("main.py"), "pass").unwrap();
        let store = Store::new(&home.path().join("px"));
        let manifest = store.install_directory(&source).unwrap();
        assert!(
            !store
                .plugin_dir(&manifest.id, &manifest.version)
                .unwrap()
                .join(".venv")
                .exists()
        );
    }
    #[test]
    fn install_and_enable_local_plugin() {
        let home = tempfile::tempdir().unwrap();
        let store = Store::new(home.path());
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/hello");
        let manifest = store.install_directory(&source).unwrap();
        assert_eq!(manifest.id, "example.hello");
        assert!(!store.load().unwrap().installed["example.hello"].enabled);
        assert!(store.install_directory(&source).is_err());
        store.set_enabled("example.hello", true).unwrap();
        assert!(store.load().unwrap().installed["example.hello"].enabled);
        store.set_enabled("example.hello", false).unwrap();
        assert!(!store.load().unwrap().installed["example.hello"].enabled);
        store.uninstall("example.hello").unwrap();
        assert!(store.load().unwrap().installed.is_empty());
    }
}
