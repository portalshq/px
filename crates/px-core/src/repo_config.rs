//! Checkout-local remote selection and Lore's native ignore policy.
//! Read these files without contacting a server, so changing the global default
//! cannot prevent access to an established checkout on another server.
use crate::error::PxError;
use std::io::Write;
use std::path::{Component, Path};

pub fn validate_repository_name(name: &str) -> Result<(), PxError> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
    {
        return Err(PxError::Other(format!("invalid repository name '{name}'")));
    }
    Ok(())
}

/// Reject remote paths that escape the checkout or address VCS/system files.
pub fn validate_root_file(path: &str) -> Result<(), PxError> {
    if path.is_empty()
        || path.contains('\\')
        || Path::new(path)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        || path.split('/').any(|part| {
            matches!(part, ".DS_Store" | ".git" | ".lore" | ".px" | "Thumbs.db")
                || part.starts_with("._")
        })
    {
        return Err(PxError::Other(format!(
            "unsafe repository-relative path '{path}'"
        )));
    }
    Ok(())
}

fn read_optional(path: &Path) -> Result<Option<String>, PxError> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn config(path: &Path) -> Result<serde_yaml::Value, PxError> {
    let file = path.join(".px/config.yaml");
    let value = match read_optional(&file)? {
        Some(text) => serde_yaml::from_str(&text)
            .map_err(|e| PxError::Other(format!("invalid .px/config.yaml: {e}")))?,
        None => serde_yaml::Value::Mapping(Default::default()),
    };
    if !value.is_mapping() {
        return Err(PxError::Other(".px/config.yaml must be a mapping".into()));
    }
    Ok(value)
}

/// Normalize a remote to its server origin. An optional repository suffix must
/// agree with the local repository, otherwise reads and pushes would diverge.
pub fn server_url(remote: &str, repository: &str) -> Result<String, PxError> {
    let mut url = reqwest::Url::parse(remote)
        .map_err(|e| PxError::Other(format!("invalid repository remote URL: {e}")))?;
    if !matches!(url.scheme(), "lore" | "lores" | "grpc" | "grpcs")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.path().trim_matches('/').is_empty() && url.path().trim_matches('/') != repository
    {
        return Err(PxError::Other(format!(
            "repository remote must be a Lore server URL or end in '/{repository}'"
        )));
    }
    url.set_path("");
    Ok(url.as_str().trim_end_matches('/').to_owned())
}

/// PX override first, then Lore's persisted server, then the supplied default.
pub fn repository_server(path: &Path, repository: &str, default: &str) -> Result<String, PxError> {
    validate_repository_name(repository)?;
    let config = config(path)?;
    if let Some(value) = config.get("remote_url") {
        let remote = value
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| {
                PxError::Other("remote_url in .px/config.yaml must be a nonempty string".into())
            })?;
        return server_url(remote, repository);
    }
    if let Some(text) = read_optional(&path.join(".lore/config.toml"))? {
        let lore: toml::Value = toml::from_str(&text)
            .map_err(|e| PxError::Other(format!("invalid .lore/config.toml: {e}")))?;
        if let Some(value) = lore.get("remote_url") {
            let remote = value
                .as_str()
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| {
                    PxError::Other(
                        "remote_url in .lore/config.toml must be a nonempty string".into(),
                    )
                })?;
            return server_url(remote, repository);
        }
    }
    server_url(default, repository)
}

fn write_atomic(path: &Path, text: &str) -> Result<(), PxError> {
    let parent = path
        .parent()
        .ok_or_else(|| PxError::Other("configuration has no parent".into()))?;
    std::fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(text.as_bytes())?;
    file.persist(path)
        .map_err(|e| PxError::Other(format!("failed to save repository configuration: {e}")))?;
    Ok(())
}

/// Keep the transport Lore actually uses aligned with a PX override. Preserve
/// unrelated Lore settings. No change is made when the checkout has no override.
pub fn apply_remote_override(path: &Path, repository: &str) -> Result<(), PxError> {
    let px = config(path)?;
    let Some(value) = px.get("remote_url") else {
        return Ok(());
    };
    let remote = value
        .as_str()
        .ok_or_else(|| PxError::Other("remote_url must be a string".into()))?;
    let server = server_url(remote, repository)?;
    let file = path.join(".lore/config.toml");
    let Some(text) = read_optional(&file)? else {
        return Ok(());
    };
    let mut lore: toml::Value = toml::from_str(&text)
        .map_err(|e| PxError::Other(format!("invalid .lore/config.toml: {e}")))?;
    let table = lore
        .as_table_mut()
        .ok_or_else(|| PxError::Other("Lore configuration must be a table".into()))?;
    if table.get("remote_url").and_then(toml::Value::as_str) != Some(&server) {
        table.insert("remote_url".into(), toml::Value::String(server));
        write_atomic(
            &file,
            &toml::to_string(&lore).map_err(|e| PxError::Other(e.to_string()))?,
        )?;
    }
    Ok(())
}

pub fn set_remote(
    path: &Path,
    repository: &str,
    remote: &str,
    source: &str,
) -> Result<(), PxError> {
    validate_repository_name(repository)?;
    let server = server_url(remote, repository)?;
    let mut px = config(path)?;
    px["repository"] = repository.into();
    px["remote_url"] = format!("{server}/{repository}").into();
    px["remote_source"] = source.into();
    write_atomic(
        &path.join(".px/config.yaml"),
        &serde_yaml::to_string(&px).map_err(|e| PxError::Other(e.to_string()))?,
    )?;
    apply_remote_override(path, repository)
}

/// Bridge .pxignore into Lore's native filter. User .loreignore rules are kept;
/// mandatory system exclusions come last so negations cannot include them.
pub fn ensure_ignore(path: &Path) -> Result<(), PxError> {
    const START: &str = "# BEGIN PX managed ignores";
    const END: &str = "# END PX managed ignores";
    let file = path.join(".loreignore");
    let original = match read_optional(&file)? {
        Some(rules) => Some(rules),
        None => read_optional(&path.join(".urcignore"))?,
    }
    .unwrap_or_default();
    let mut user = original.clone();
    if let Some(start) = user.find(START) {
        let end = user[start..]
            .find(END)
            .ok_or_else(|| PxError::Other("unterminated PX ignore block in .loreignore".into()))?
            + start
            + END.len();
        user.replace_range(start..end, "");
    }
    let custom = read_optional(&path.join(".pxignore"))?.unwrap_or_default();
    let updated = format!(
        "{}\n{START}\n{custom}\n.DS_Store\n._*\nThumbs.db\nDesktop.ini\n.git\n.px\n.lore\n{END}\n",
        user.trim_end()
    );
    if updated != original {
        write_atomic(&file, &updated)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ignores_are_idempotent_and_preserve_user_patterns() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(root.join(".loreignore"), "*.scratch\n").unwrap();
        std::fs::write(root.join(".pxignore"), "cache/\n!cache/keep.txt\n").unwrap();
        ensure_ignore(root).unwrap();
        let first = std::fs::read_to_string(root.join(".loreignore")).unwrap();
        ensure_ignore(root).unwrap();
        assert_eq!(
            first,
            std::fs::read_to_string(root.join(".loreignore")).unwrap()
        );
        assert!(first.contains("*.scratch"));
        assert!(first.contains("cache/\n!cache/keep.txt"));
        assert!(first.contains(".DS_Store"));
        std::fs::write(root.join(".pxignore"), "new-cache/\n").unwrap();
        ensure_ignore(root).unwrap();
        let updated = std::fs::read_to_string(root.join(".loreignore")).unwrap();
        assert!(!updated.contains("!cache/keep.txt"));
        assert!(updated.contains("new-cache/"));
    }
    #[test]
    fn paths_and_repository_names_cannot_escape_checkout() {
        for path in [
            "../toystory/character/hero.yaml",
            "/tmp/hero.yaml",
            ".px/config.yaml",
            "character/.DS_Store",
            "character/../hero.yaml",
            "character\\hero.yaml",
        ] {
            assert!(validate_root_file(path).is_err(), "{path}");
        }
        for name in ["", "../toys", "bears/toys", "."] {
            assert!(validate_repository_name(name).is_err());
        }
        validate_root_file("character/hero.yaml").unwrap();
    }
    #[test]
    fn checkout_remote_precedes_global_and_malformed_config_fails() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir(root.join(".lore")).unwrap();
        std::fs::write(
            root.join(".lore/config.toml"),
            "remote_url = 'lore://server-a:41337'\n[file]\ndirect_io = false\n",
        )
        .unwrap();
        assert_eq!(
            repository_server(root, "bears", "lore://server-b:41337").unwrap(),
            "lore://server-a:41337"
        );
        set_remote(root, "bears", "lores://custom.example/bears", "custom").unwrap();
        assert_eq!(
            repository_server(root, "bears", "lore://server-b:41337").unwrap(),
            "lores://custom.example"
        );
        let lore = std::fs::read_to_string(root.join(".lore/config.toml")).unwrap();
        assert!(lore.contains("direct_io = false"));
        std::fs::write(root.join(".px/config.yaml"), "remote_url: [bad]\n").unwrap();
        assert!(repository_server(root, "bears", "lore://server-b:41337").is_err());
    }
    #[test]
    fn remote_identity_and_credentials_are_validated() {
        for value in [
            "lore://host/toystory",
            "lore://user:password@host/bears",
            "lore://host/bears?token=secret",
            "http://host",
            "lore://host/a/b",
        ] {
            assert!(server_url(value, "bears").is_err());
        }
    }
}

/// Canonical identity is taken from the manifest, including during temporary
/// initialization/clone operations where the directory name is not the repo.
pub fn checkout_repository(path: &Path) -> Result<String, PxError> {
    let text = std::fs::read_to_string(path.join("repository.yaml"))?;
    let value: serde_yaml::Value = serde_yaml::from_str(&text)
        .map_err(|e| PxError::Other(format!("invalid repository.yaml: {e}")))?;
    let id = value
        .get("id")
        .and_then(serde_yaml::Value::as_str)
        .ok_or_else(|| PxError::Other("repository.yaml has no PX identity".into()))?;
    let uri: crate::uri::PxUri = id.parse()?;
    validate_repository_name(&uri.repository)?;
    Ok(uri.repository)
}

#[derive(Debug)]
pub struct RemoteMigration {
    pub repository: String,
    pub previous: String,
    pub update: bool,
}

/// Existing concrete remotes are pinned unless explicitly marked as following
/// the provider. This preserves legacy mixed-server checkouts. A manually edited
/// override is protected even when its source field still says `default`.
pub fn plan_remote_migration(
    home: &Path,
    old_default: &str,
    force: bool,
) -> Result<Vec<RemoteMigration>, PxError> {
    let mut result = Vec::new();
    if !home.exists() {
        return Ok(result);
    }
    for entry in std::fs::read_dir(home)? {
        let entry = entry?;
        let root = entry.path();
        if entry.file_type()?.is_symlink()
            || !root.is_dir()
            || !root.join("repository.yaml").is_file()
        {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        validate_repository_name(&name)?;
        if checkout_repository(&root)? != name {
            return Err(PxError::Other(format!(
                "repository identity does not match directory '{name}'"
            )));
        }
        let px = config(&root)?;
        let server = repository_server(&root, &name, old_default)?;
        let has_remote = px.get("remote_url").is_some() || root.join(".lore/config.toml").exists();
        let follows_default = px.get("remote_source").and_then(serde_yaml::Value::as_str)
            == Some("default")
            && crate::provider::http::same_server(&server, old_default);
        result.push(RemoteMigration {
            repository: name.clone(),
            previous: format!("{server}/{name}"),
            update: force || !has_remote || follows_default,
        });
    }
    result.sort_by(|a, b| a.repository.cmp(&b.repository));
    Ok(result)
}

#[cfg(test)]
mod migration_tests {
    use super::*;
    #[test]
    fn provider_migration_preserves_pinned_and_manually_changed_remotes() {
        let home = tempfile::tempdir().unwrap();
        for name in ["pinned", "default", "custom", "bare"] {
            let root = home.path().join(name);
            std::fs::create_dir_all(&root).unwrap();
            std::fs::write(
                root.join("repository.yaml"),
                format!("id: px://{name}/world/{name}\n"),
            )
            .unwrap();
        }
        set_remote(
            &home.path().join("pinned"),
            "pinned",
            "lore://a:41337",
            "custom",
        )
        .unwrap();
        set_remote(
            &home.path().join("default"),
            "default",
            "lore://a:41337",
            "default",
        )
        .unwrap();
        set_remote(
            &home.path().join("custom"),
            "custom",
            "lore://custom:41337",
            "default",
        )
        .unwrap();
        let plan = plan_remote_migration(home.path(), "lore://a:41337", false).unwrap();
        assert_eq!(
            plan.iter()
                .filter(|p| p.update)
                .map(|p| p.repository.as_str())
                .collect::<Vec<_>>(),
            ["bare", "default"]
        );
        assert!(
            plan_remote_migration(home.path(), "lore://a:41337", true)
                .unwrap()
                .iter()
                .all(|p| p.update)
        );
    }
}
