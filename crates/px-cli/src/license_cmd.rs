use anyhow::Result;
use px_cli::LicenseCmd;
use px_plugin::license::LicenseStore;
use std::path::Path;

pub fn run(base_dir: &Path, command: LicenseCmd) -> Result<()> {
    let store = LicenseStore::new(base_dir);
    let now = chrono::Utc::now().timestamp();
    let output = match command {
        LicenseCmd::Status => serde_json::to_value(store.status(now).map_err(anyhow::Error::msg)?)?,
        LicenseCmd::Add { token_file } => {
            let claims = store.add(&token_file, now).map_err(anyhow::Error::msg)?;
            serde_json::json!({"plugin_id":claims.plugin_id,"major":claims.major,"added":true})
        }
        LicenseCmd::Remove { id } => {
            store.remove(&id).map_err(anyhow::Error::msg)?;
            serde_json::json!({"plugin_id":id,"removed":true})
        }
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
