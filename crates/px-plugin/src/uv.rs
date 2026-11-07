//! Versioned official uv bootstrap; no system Python or shell-profile edits.
//! Installer behavior: https://docs.astral.sh/uv/reference/installer/
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{io::AsyncReadExt, process::Command};

pub const PINNED_UV_VERSION: &str = "0.10.12";
const SH_SHA256: &str = "2dbc8204431a43a30f5396f3bb94d3f4505a2aabd4d35a9f75d5d9d6cfa81528";
const PS1_SHA256: &str = "688fb18494b49a651726c3830060aae8f2b1b84864b66b0cfdfbbae93e72a38f";

pub struct UvInstaller {
    bin_dir: PathBuf,
    override_path: Option<PathBuf>,
}
impl UvInstaller {
    pub fn new(px_home: &Path) -> Self {
        Self {
            bin_dir: px_home.join("tools/uv/bin"),
            override_path: None,
        }
    }
    pub fn with_override(mut self, path: PathBuf) -> Self {
        self.override_path = Some(path);
        self
    }
    pub async fn verify(path: &Path) -> Result<(), String> {
        let mut command = Command::new(path);
        command.arg("--version");
        let output = run(command, Duration::from_secs(5)).await?;
        let text = std::str::from_utf8(&output).map_err(|_| "invalid uv version output")?;
        let mut parts = text.split_whitespace();
        if parts.next() != Some("uv") {
            return Err("executable did not identify itself as uv".into());
        }
        let version = semver::Version::parse(parts.next().ok_or("missing uv version")?)
            .map_err(|_| "invalid uv version")?;
        let minimum = semver::Version::parse(PINNED_UV_VERSION).map_err(|e| e.to_string())?;
        if version < minimum {
            return Err(format!("uv {PINNED_UV_VERSION} or newer is required"));
        }
        Ok(())
    }
    pub async fn ensure(&self) -> Result<PathBuf, String> {
        if let Some(path) = &self.override_path {
            Self::verify(path).await?;
            return Ok(path.clone());
        }
        let binary = self
            .bin_dir
            .join(if cfg!(windows) { "uv.exe" } else { "uv" });
        if Self::verify(&binary).await.is_ok() {
            return Ok(binary);
        }
        if Self::verify(Path::new("uv")).await.is_ok() {
            return Ok(PathBuf::from("uv"));
        }
        self.install().await?;
        Self::verify(&binary).await?;
        Ok(binary)
    }
    async fn install(&self) -> Result<(), String> {
        let (extension, checksum) = if cfg!(windows) {
            ("ps1", PS1_SHA256)
        } else {
            ("sh", SH_SHA256)
        };
        let url = format!("https://astral.sh/uv/{PINNED_UV_VERSION}/install.{extension}");
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| e.to_string())?;
        let mut response = client
            .get(url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
            if bytes.len().saturating_add(chunk.len()) > 1024 * 1024 {
                return Err("uv installer exceeds size limit".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        verify_installer(&bytes, checksum)?;
        std::fs::create_dir_all(&self.bin_dir).map_err(|e| e.to_string())?;
        let mut script = tempfile::Builder::new()
            .prefix(".uv-install-")
            .suffix(&format!(".{extension}"))
            .tempfile_in(&self.bin_dir)
            .map_err(|e| e.to_string())?;
        script.write_all(&bytes).map_err(|e| e.to_string())?;
        script.flush().map_err(|e| e.to_string())?;
        let mut command = if cfg!(windows) {
            let mut cmd = Command::new("powershell");
            cmd.args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(script.path());
            cmd
        } else {
            let mut cmd = Command::new("sh");
            cmd.arg(script.path());
            cmd
        };
        command
            .env("UV_UNMANAGED_INSTALL", &self.bin_dir)
            .env("UV_NO_MODIFY_PATH", "1")
            .env("UV_NO_PROGRESS", "1");
        run(command, Duration::from_secs(120)).await?;
        Ok(())
    }
}
fn verify_installer(bytes: &[u8], expected: &str) -> Result<(), String> {
    if hex::encode(Sha256::digest(bytes)) != expected {
        return Err("uv installer checksum mismatch".into());
    }
    Ok(())
}
async fn run(mut command: Command, timeout: Duration) -> Result<Vec<u8>, String> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| e.to_string())?;
    let out = child.stdout.take().ok_or("missing uv output")?;
    let err = child.stderr.take().ok_or("missing uv errors")?;
    let read = |stream: Box<dyn tokio::io::AsyncRead + Unpin + Send>| async move {
        let mut bytes = Vec::new();
        stream
            .take(65537)
            .read_to_end(&mut bytes)
            .await
            .map_err(|e| e.to_string())?;
        if bytes.len() > 65536 {
            return Err("uv output exceeds limit".to_string());
        }
        Ok::<_, String>(bytes)
    };
    let future = async {
        let (out, err, status) =
            tokio::try_join!(read(Box::new(out)), read(Box::new(err)), async {
                child.wait().await.map_err(|e| e.to_string())
            })?;
        if !status.success() {
            return Err(format!(
                "uv command failed: {}",
                String::from_utf8_lossy(&err)
            ));
        }
        Ok(out)
    };
    tokio::time::timeout(timeout, future)
        .await
        .map_err(|_| "uv command timed out".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installer_integrity_is_required_before_execution() {
        let script = b"official installer bytes";
        assert!(verify_installer(script, &hex::encode(Sha256::digest(script))).is_ok());
        assert!(verify_installer(b"modified", &hex::encode(Sha256::digest(script))).is_err());
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn explicit_uv_path_is_checked_and_old_or_fake_binaries_are_rejected() {
        use std::os::unix::fs::PermissionsExt;
        let home = tempfile::tempdir().unwrap();
        let binary = home.path().join("uv");
        std::fs::write(&binary, "#!/bin/sh\necho 'uv 0.10.12 (test)'\n").unwrap();
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();
        let installer = UvInstaller::new(home.path()).with_override(binary.clone());
        assert_eq!(installer.ensure().await.unwrap(), binary);
        std::fs::write(&binary, "#!/bin/sh\necho 'uv 0.1.0'\n").unwrap();
        assert!(installer.ensure().await.is_err());
        std::fs::write(&binary, "#!/bin/sh\necho 'something else'\n").unwrap();
        assert!(installer.ensure().await.is_err());
    }
}
