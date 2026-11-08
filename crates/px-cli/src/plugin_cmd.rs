use anyhow::{Result, bail};
use px_cli::PluginCmd;
use px_plugin::harness::HarnessAdapter;
use px_plugin::{registry::Store, resolve};
use std::path::Path;

fn skill_paths(store: &Store, id: &str) -> Result<Vec<std::path::PathBuf>> {
    let manifest = store.manifest(id)?;
    let root = store.plugin_dir(id, &manifest.version)?;
    Ok(manifest.skills.iter().map(|path| root.join(path)).collect())
}

fn skill_adapters() -> Result<Vec<px_plugin::harness::DirectoryAdapter>> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .ok_or_else(|| anyhow::anyhow!("home directory is unavailable"))?;
    Ok(px_plugin::harness::detect(Path::new(&home)))
}

fn install_skills(store: &Store, id: &str) -> Result<()> {
    let skills = skill_paths(store, id)?;
    if skills.is_empty() {
        return Ok(());
    }
    let adapters = skill_adapters()?;
    let mut installed: Vec<(usize, Vec<String>)> = Vec::new();
    for (index, adapter) in adapters.iter().enumerate() {
        let names: Vec<String> = skills
            .iter()
            .filter_map(|path| path.file_name().and_then(|name| name.to_str()))
            .filter(|name| !adapter.skills_dir.join(name).exists())
            .map(str::to_owned)
            .collect();
        if let Err(error) = adapter.install_skills(id, &skills) {
            // The current adapter may have installed earlier skills before a failure.
            let _ = adapter.uninstall_skills(id, &names);
            for (previous, names) in installed.into_iter().rev() {
                let _ = adapters[previous].uninstall_skills(id, &names);
            }
            return Err(error.into());
        }
        installed.push((index, names));
    }
    Ok(())
}

fn uninstall_skills(store: &Store, id: &str) -> Result<()> {
    let skills = skill_paths(store, id)?;
    if skills.is_empty() {
        return Ok(());
    }
    let names = skills
        .iter()
        .filter_map(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned)
        })
        .collect::<Vec<_>>();
    for adapter in skill_adapters()? {
        adapter.uninstall_skills(id, &names)?;
    }
    Ok(())
}

fn recover_upgrade(store: &Store) -> Result<()> {
    let Some(pending) = store.pending_upgrade()? else {
        return Ok(());
    };
    let candidate_root = store.plugin_dir(&pending.id, &pending.target_version)?;
    if !candidate_root.exists() {
        store.rollback_pending_upgrade(&pending)?;
        store.clear_upgrade_journal()?;
        return Ok(());
    }
    let candidate: px_plugin::manifest::PluginManifest =
        serde_yaml::from_slice(&std::fs::read(candidate_root.join("plugin.yaml"))?)?;
    candidate.validate(&Default::default())?;
    if candidate.id != pending.id || candidate.version != pending.target_version {
        bail!("upgrade recovery manifest does not match the journal");
    }
    store.rollback_pending_upgrade(&pending)?;
    if pending.previous.enabled {
        let previous = store.manifest(&pending.id)?;
        let previous_names: std::collections::BTreeSet<_> = previous
            .skills
            .iter()
            .filter_map(|s| Path::new(s).file_name().and_then(|n| n.to_str()))
            .collect();
        let new_only: Vec<String> = candidate
            .skills
            .iter()
            .filter_map(|s| Path::new(s).file_name().and_then(|n| n.to_str()))
            .filter(|n| !previous_names.contains(n))
            .map(str::to_owned)
            .collect();
        for adapter in skill_adapters()? {
            let owned: Vec<_> = new_only
                .iter()
                .filter(|name| {
                    std::fs::read_to_string(adapter.skills_dir.join(name).join(".px-owner"))
                        .is_ok_and(|owner| owner == pending.id)
                })
                .cloned()
                .collect();
            adapter.uninstall_skills(&pending.id, &owned)?;
        }
        install_skills(store, &pending.id)?;
    }
    if !std::fs::symlink_metadata(&candidate_root)?
        .file_type()
        .is_dir()
    {
        bail!("upgrade recovery package is not a directory");
    }
    std::fs::remove_dir_all(&candidate_root)?;
    store.clear_upgrade_journal()?;
    Ok(())
}

fn repository_pins(
    base_dir: &Path,
    input: &serde_json::Value,
) -> Result<Option<std::collections::BTreeMap<String, String>>> {
    let Some(repository) = input.get("repository").and_then(serde_json::Value::as_str) else {
        return Ok(None);
    };
    px_core::repo_config::validate_repository_name(repository)?;
    let path = base_dir.join(repository).join(".px/config.yaml");
    let data = match std::fs::read_to_string(path) {
        Ok(data) => data,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let config: serde_yaml::Value = serde_yaml::from_str(&data)?;
    config
        .get("plugins")
        .map(|value| serde_yaml::from_value(value.clone()).map_err(Into::into))
        .transpose()
}

fn eligible(
    store: &Store,
    pins: Option<&std::collections::BTreeMap<String, String>>,
) -> Result<resolve::Resolution> {
    let Some(platform) = platform() else {
        bail!("unsupported platform");
    };
    resolve::effective(store, env!("CARGO_PKG_VERSION"), platform, pins).map_err(anyhow::Error::msg)
}

fn platform() -> Option<&'static str> {
    match (std::env::consts::ARCH, std::env::consts::OS) {
        ("aarch64", "macos") => Some("aarch64-apple-darwin"),
        ("x86_64", "macos") => Some("x86_64-apple-darwin"),
        ("x86_64", "linux") => Some("x86_64-unknown-linux-gnu"),
        ("aarch64", "linux") => Some("aarch64-unknown-linux-gnu"),
        ("x86_64", "windows") => Some("x86_64-pc-windows-msvc"),
        _ => None,
    }
}
pub fn run(base_dir: &Path, cmd: PluginCmd) -> Result<()> {
    let store = Store::new(base_dir);
    let _lock = store.acquire_lock()?;
    recover_upgrade(&store)?;
    match cmd {
        PluginCmd::List => println!("{}", serde_json::to_string_pretty(&store.load()?)?),
        PluginCmd::Info { id } => {
            println!("{}", serde_json::to_string_pretty(&store.manifest(&id)?)?)
        }
        PluginCmd::Install { path, blake3 } => {
            let manifest = if path.starts_with("https://") || path.starts_with("http://") {
                let hash = blake3.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("URL installs require --blake3 blake3:<digest>")
                })?;
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                runtime
                    .block_on(px_plugin::index::install_url(&store, &path, hash))
                    .map_err(anyhow::Error::msg)?
            } else {
                if blake3.is_some() {
                    bail!("--blake3 is used with URL installs; local packages must be directories");
                }
                store.install_directory(Path::new(&path))?
            };
            println!("{}", serde_json::to_string_pretty(&manifest)?);
        }
        PluginCmd::Upgrade { path } => {
            let platform = platform().ok_or_else(|| anyhow::anyhow!("unsupported platform"))?;
            let (manifest, previous) =
                store.upgrade_directory(Path::new(&path), env!("CARGO_PKG_VERSION"), platform)?;
            if previous.enabled {
                let update = || -> Result<()> {
                    let old_root = store.plugin_dir(&manifest.id, &previous.version)?;
                    let old: px_plugin::manifest::PluginManifest =
                        serde_yaml::from_slice(&std::fs::read(old_root.join("plugin.yaml"))?)?;
                    let new_names: std::collections::BTreeSet<_> = manifest
                        .skills
                        .iter()
                        .filter_map(|s| Path::new(s).file_name().and_then(|n| n.to_str()))
                        .collect();
                    let old_only: Vec<String> = old
                        .skills
                        .iter()
                        .filter_map(|s| Path::new(s).file_name().and_then(|n| n.to_str()))
                        .filter(|n| !new_names.contains(n))
                        .map(str::to_owned)
                        .collect();
                    install_skills(&store, &manifest.id)?;
                    for adapter in skill_adapters()? {
                        adapter.uninstall_skills(&manifest.id, &old_only)?;
                    }
                    Ok(())
                };
                if let Err(error) = update() {
                    recover_upgrade(&store).map_err(|recovery| {
                        anyhow::anyhow!("upgrade failed: {error}; recovery failed: {recovery}")
                    })?;
                    return Err(error);
                }
            }
            store.clear_upgrade_journal()?;
            println!(
                "{}",
                serde_json::json!({"upgraded":manifest.id,"version":manifest.version,"enabled":previous.enabled})
            );
        }
        PluginCmd::Uninstall { id } => {
            uninstall_skills(&store, &id)?;
            store.uninstall(&id)?;
            println!("{}", serde_json::json!({"uninstalled": id}));
        }
        PluginCmd::Enable { id, repo } => {
            if let Some(repository) = &repo {
                px_core::repo_config::validate_repository_name(repository)?;
                store.set_repository_enabled(&base_dir.join(repository), &id, true)?;
            } else {
                install_skills(&store, &id)?;
                store.set_enabled(&id, true)?;
            }
            println!("{}", serde_json::json!({"enabled": id}));
        }
        PluginCmd::Disable { id, repo } => {
            if let Some(repository) = &repo {
                px_core::repo_config::validate_repository_name(repository)?;
                store.set_repository_enabled(&base_dir.join(repository), &id, false)?;
            } else {
                uninstall_skills(&store, &id)?;
                store.set_enabled(&id, false)?;
            }
            println!("{}", serde_json::json!({"disabled": id}));
        }
        PluginCmd::Doctor => {
            let result = eligible(&store, None)?;
            let enabled: Vec<_> = result.enabled.iter().map(|p| &p.manifest.id).collect();
            let excluded = result
                .excluded
                .iter()
                .map(|(id, reason)| (id, format!("{reason:?}")))
                .collect::<std::collections::BTreeMap<_, _>>();
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({"enabled": enabled, "excluded": excluded})
                )?
            );
        }
        PluginCmd::Tools { format } => {
            if format != "json" {
                bail!("plugin tools supports JSON only");
            }
            let result = eligible(&store, None)?;
            let mut tools = Vec::new();
            for plugin in result.enabled {
                for tool in &plugin.manifest.tools {
                    tools.push(serde_json::json!({
                        "name": format!("px_{}_{}", plugin.manifest.namespace, tool.name.replace('-', "_")),
                        "id": plugin.manifest.id, "tool": tool.name, "description": tool.description,
                        "input_schema": tool.input_schema, "annotations": tool.annotations, "timeout_ms": tool.timeout_ms,
                    }));
                }
            }
            println!("{}", serde_json::to_string(&tools)?);
        }
        PluginCmd::Call { id, tool, input } => {
            let input: serde_json::Value = serde_json::from_str(&input)?;
            let pins = repository_pins(base_dir, &input)?;
            let plugin = eligible(&store, pins.as_ref())?
                .enabled
                .into_iter()
                .find(|p| p.manifest.id == id)
                .ok_or_else(|| anyhow::anyhow!("plugin is not enabled, compatible, or entitled"))?;
            let spec = plugin
                .manifest
                .tools
                .iter()
                .find(|candidate| candidate.name == tool)
                .ok_or_else(|| anyhow::anyhow!("unknown plugin tool"))?;
            let validator = jsonschema::validator_for(&spec.input_schema)?;
            if !validator.is_valid(&input) {
                bail!("plugin input does not match its schema");
            }
            let request = px_plugin::protocol::Request {
                context: px_plugin::protocol::Context {
                    base_dir: base_dir.to_string_lossy().into_owned(),
                    px_binary: Some(std::env::current_exe()?.to_string_lossy().into_owned()),
                    repository: input
                        .get("repository")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned),
                    branch: input
                        .get("branch")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned),
                    session: input
                        .get("session")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned),
                },
                input,
                entitlement: plugin
                    .entitlement
                    .as_ref()
                    .map(serde_json::to_value)
                    .transpose()?,
            };
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            let timeout = std::time::Duration::from_millis(spec.timeout_ms);
            // Plugin processes may invoke PX recursively; never hold the
            // registry lock while running a child process.
            drop(_lock);
            let result = match plugin.manifest.kind {
                px_plugin::manifest::PluginKind::Executable => runtime.block_on(
                    px_plugin::protocol::call(&plugin.entrypoint, &tool, &request, timeout),
                ),
                px_plugin::manifest::PluginKind::UvPython => {
                    let project =
                        store.plugin_dir(&plugin.manifest.id, &plugin.manifest.version)?;
                    let mut installer = px_plugin::uv::UvInstaller::new(base_dir);
                    if let Some(path) = std::env::var_os("PX_UV_PATH") {
                        installer = installer.with_override(path.into());
                    }
                    let uv = runtime
                        .block_on(installer.ensure())
                        .map_err(anyhow::Error::msg)?;
                    runtime.block_on(px_plugin::protocol::call_uv(
                        &uv,
                        &project,
                        &plugin.entrypoint,
                        &tool,
                        &request,
                        timeout,
                    ))
                }
            }
            .map_err(anyhow::Error::msg)?;
            match result {
                px_plugin::protocol::Response::Output { output } => {
                    println!("{}", serde_json::to_string(&output)?)
                }
                px_plugin::protocol::Response::Error { error } => {
                    bail!("{}: {}", error.code, error.message)
                }
            }
        }
    }
    Ok(())
}
