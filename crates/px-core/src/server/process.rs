// SPDX-FileCopyrightText: 2026 Digital Creations
// SPDX-License-Identifier: MIT
//! Lore server process management
//!
//! Cross-platform detached process launch and lifecycle management for Lore server.

use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

/// Which process owns the TCP listener PX expects its Lore server to own.
#[derive(Debug)]
pub enum ListenerIdentity {
    /// Nothing is listening on the port.
    NoListener,
    /// The listener's command line contains `--config <expected config dir>`
    /// — it is the Lore server for this PX home.
    Ours(u32),
    /// A foreign `loreserver` is bound to the port with a different config
    /// directory (e.g. a stale test instance or another PX home).
    ForeignLore {
        pid: u32,
        cmdline: String,
        config_dir: String,
    },
    /// A non-loreserver process owns the port — never killed automatically.
    ForeignProcess { pid: u32, cmdline: String },
    /// The listener could not be inspected (missing tools or permissions).
    Unknown,
}

/// Lore server process manager
pub struct LoreProcessManager {
    px_home: std::path::PathBuf,
    log_path: std::path::PathBuf,
    config_path: std::path::PathBuf,
}

impl LoreProcessManager {
    /// Create a new Lore process manager
    pub fn new(px_home: &Path) -> Self {
        let config_path = px_home.join("lore").join("config").join("local.toml");
        let log_path = px_home.join("lore").join("logs").join("loreserver.log");

        Self {
            px_home: px_home.to_path_buf(),
            config_path,
            log_path,
        }
    }

    /// Expected `--config` directory passed to `loreserver` at spawn
    /// (`<px_home>/lore/config`, the parent of `local.toml`).
    pub fn expected_config_dir(&self) -> std::path::PathBuf {
        self.px_home.join("lore").join("config")
    }

    /// Resolve who owns the TCP listener on `port`.
    ///
    /// Reads the listener PID via `lsof`, then inspects the process command
    /// line for `--config <expected_config_dir>`. Path comparison canonicalizes
    /// both sides so `/var` vs `/private/var` (macOS) and symlinked temp dirs
    /// resolve to the same location.
    #[cfg(unix)]
    pub fn listener_identity(&self, port: u16) -> ListenerIdentity {
        let Some(pid) = listener_pid(port) else {
            return ListenerIdentity::NoListener;
        };
        let Some(cmdline) = process_cmdline(pid) else {
            tracing::warn!(pid, port, "could not read command line of port listener");
            return ListenerIdentity::Unknown;
        };
        if !cmdline.contains("loreserver") {
            return ListenerIdentity::ForeignProcess { pid, cmdline };
        }
        let expected_dir = self.expected_config_dir();
        if cmdline_has_config_dir(&cmdline, &expected_dir) {
            ListenerIdentity::Ours(pid)
        } else {
            ListenerIdentity::ForeignLore {
                pid,
                cmdline: cmdline.clone(),
                config_dir: extract_config_dir(&cmdline).unwrap_or_else(|| "<none>".into()),
            }
        }
    }

    #[cfg(not(unix))]
    pub fn listener_identity(&self, _port: u16) -> ListenerIdentity {
        ListenerIdentity::Unknown
    }

    /// Returns the PID of the loreserver listening on `port` when it belongs
    /// to this PX home, `None` when the port is held by a foreign server or
    /// nothing is listening. Convenience wrapper for callers that only need
    /// the "is it ours" answer.
    #[allow(dead_code)]
    pub fn our_listener_pid(&self, port: u16) -> Option<u32> {
        match self.listener_identity(port) {
            ListenerIdentity::Ours(pid) => Some(pid),
            _ => None,
        }
    }

    /// Start Lore server as a detached background process
    #[allow(clippy::needless_return)]
    pub fn start(&self) -> Result<u32> {
        // Ensure log directory exists
        if let Some(parent) = self.log_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create log directory")?;
        }

        // Verify configuration exists
        if !self.config_path.exists() {
            anyhow::bail!(
                "Lore configuration not found at '{}'. \
                 Run 'px init' or ensure configuration generation has been completed \
                 before starting the Lore server.",
                self.config_path.display()
            );
        }

        // Open log file for output
        let log_file =
            std::fs::File::create(&self.log_path).context("Failed to create log file")?;

        tracing::info!(
            config = %self.config_path.display(),
            log = %self.log_path.display(),
            "Starting Lore server in detached mode"
        );

        // Launch Lore server with configuration in detached mode
        #[cfg(unix)]
        {
            // Spawn lore server directly and drop the Child handle immediately.
            // The process will be re-parented to init when the parent exits.
            // We rely on health checks rather than PID tracking for server status.
            let child = Command::new("loreserver")
                .arg("--config")
                .arg(self.config_path.parent().unwrap_or(&self.config_path))
                .stdout(Stdio::from(log_file.try_clone()?))
                .stderr(Stdio::from(log_file))
                .spawn()
                .context("Failed to start Lore server. Is loreserver installed and on PATH?")?;

            let pid = child.id();
            tracing::info!(pid, "Lore server started in detached mode");
            return Ok(pid);
        }

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            use windows_sys::Win32::System::Threading::DETACHED_PROCESS;

            let child = Command::new("loreserver")
                .arg("--config")
                .arg(self.config_path.parent().unwrap_or(&self.config_path))
                .stdout(Stdio::from(log_file.try_clone()?))
                .stderr(Stdio::from(log_file))
                .creation_flags(DETACHED_PROCESS)
                .spawn()
                .context("Failed to start Lore server. Is loreserver installed and on PATH?")?;

            let pid = child.id();
            tracing::info!(pid, "Lore server started in detached mode");
            return Ok(pid);
        }

        #[cfg(not(any(unix, windows)))]
        {
            let child = Command::new("loreserver")
                .arg("--config")
                .arg(self.config_path.parent().unwrap_or(&self.config_path))
                .stdout(Stdio::from(log_file.try_clone()?))
                .stderr(Stdio::from(log_file))
                .spawn()
                .context("Failed to start Lore server. Is loreserver installed and on PATH?")?;

            let pid = child.id();
            tracing::info!(pid, "Lore server started");
            Ok(pid)
        }
    }

    /// Stop Lore server by PID
    pub fn stop(pid: u32) -> Result<()> {
        tracing::info!(pid, "Stopping Lore server");

        #[cfg(unix)]
        {
            use nix::sys::signal::kill;
            use nix::unistd::Pid;

            kill(Pid::from_raw(pid as i32), nix::sys::signal::Signal::SIGTERM)
                .context("Failed to send SIGTERM to Lore server")?;
        }

        #[cfg(windows)]
        {
            use windows_sys::Win32::Foundation::CloseHandle;
            use windows_sys::Win32::System::Threading::{
                OpenProcess, PROCESS_TERMINATE, TerminateProcess,
            };

            unsafe {
                let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
                if handle == std::ptr::null_mut() {
                    anyhow::bail!("Failed to open process handle for PID {}", pid);
                }
                let result = TerminateProcess(handle, 1);
                CloseHandle(handle);
                if result == 0 {
                    anyhow::bail!("Failed to terminate process with PID {}", pid);
                }
            }
        }

        tracing::info!(pid, "Lore server stopped");
        Ok(())
    }

    /// Check if Lore server is running by PID
    pub fn is_running(pid: u32) -> bool {
        #[cfg(unix)]
        {
            use nix::sys::signal::kill;
            use nix::unistd::Pid;

            kill(Pid::from_raw(pid as i32), None).is_ok()
        }

        #[cfg(windows)]
        {
            use windows_sys::Win32::Foundation::CloseHandle;
            use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION};

            unsafe {
                let handle = OpenProcess(PROCESS_QUERY_INFORMATION, 0, pid);
                if handle == std::ptr::null_mut() {
                    return false;
                }
                CloseHandle(handle);
                true
            }
        }
    }

    /// Perform health check on Lore server
    pub async fn health_check(port: u16, timeout: Duration) -> Result<bool> {
        let url = format!("http://127.0.0.1:{}/health_check", port);

        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .context("Failed to build HTTP client for Lore health check")?;

        let response = client.get(&url).send().await;

        match response {
            Ok(resp) => {
                let is_healthy = resp.status().is_success();
                if is_healthy {
                    tracing::debug!(
                        port,
                        url = %url,
                        "Lore server health check passed"
                    );
                } else {
                    tracing::warn!(
                        port,
                        url = %url,
                        status = %resp.status(),
                        "Lore server health check returned non-success status"
                    );
                }
                Ok(is_healthy)
            }
            Err(e) => {
                if e.is_timeout() {
                    tracing::debug!(
                        port,
                        url = %url,
                        timeout_ms = timeout.as_millis(),
                        "Lore server health check timed out after {:?} — server may still be starting",
                        timeout
                    );
                } else if e.is_connect() {
                    tracing::debug!(
                        port,
                        url = %url,
                        "Lore server health check connection refused — server is not listening on port {}",
                        port
                    );
                } else {
                    tracing::debug!(
                        port,
                        url = %url,
                        error = %e,
                        "Lore server health check failed"
                    );
                }
                Ok(false)
            }
        }
    }

    /// Wait for Lore server to become healthy
    pub async fn wait_for_healthy(
        port: u16,
        timeout: Duration,
        retry_interval: Duration,
    ) -> Result<()> {
        let start = std::time::Instant::now();
        let mut attempt = 0;

        while start.elapsed() < timeout {
            attempt += 1;
            tracing::debug!(
                attempt,
                elapsed_ms = start.elapsed().as_millis(),
                timeout_ms = timeout.as_millis(),
                "Health check attempt for Lore server on port {}",
                port
            );

            if Self::health_check(port, retry_interval).await? {
                tracing::info!(
                    attempt,
                    elapsed_ms = start.elapsed().as_millis(),
                    port,
                    "Lore server became healthy after {} attempts ({:?})",
                    attempt,
                    start.elapsed()
                );
                return Ok(());
            }
            tokio::time::sleep(retry_interval).await;
        }

        anyhow::bail!(
            "Lore server on port {} did not become healthy within {:?} ({} attempts). \
             Possible causes: server crash during startup, port conflict, \
             configuration error, or insufficient resources. \
             Check logs for details.",
            port,
            timeout,
            attempt
        );
    }

    /// Get the log file path
    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    /// Get the configuration path
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }
}

/// PID of the process listening on a TCP port, via `lsof`.
#[cfg(unix)]
fn listener_pid(port: u16) -> Option<u32> {
    let output = Command::new("lsof")
        .args(["-nP", &format!("-iTCP:{port}"), "-sTCP:LISTEN", "-t"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()?
        .trim()
        .parse()
        .ok()
}

/// Command line of a process — `/proc` on Linux, `ps` elsewhere.
#[cfg(unix)]
fn process_cmdline(pid: u32) -> Option<String> {
    #[cfg(target_os = "linux")]
    if let Ok(bytes) = std::fs::read(format!("/proc/{pid}/cmdline")) {
        return Some(
            bytes
                .iter()
                .map(|b| if *b == 0 { ' ' } else { *b as char })
                .collect(),
        );
    }
    let output = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "command="])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Canonicalize a path for comparison; falls back to the raw path when the
/// target does not exist on disk.
fn normalize_dir(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}

/// Extract the `--config <dir>` value from a command line.
fn extract_config_dir(cmdline: &str) -> Option<String> {
    let mut parts = cmdline.split_whitespace();
    while let Some(part) = parts.next() {
        if part == "--config" {
            return parts.next().map(|s| s.trim_end_matches('/').to_string());
        }
        if let Some(dir) = part.strip_prefix("--config=") {
            return Some(dir.trim_end_matches('/').to_string());
        }
    }
    None
}

/// Whether a process command line was started with `--config <expected_dir>`.
/// Compares canonicalized paths so `/var` vs `/private/var` and symlinked
/// temp dirs match correctly.
fn cmdline_has_config_dir(cmdline: &str, expected_dir: &Path) -> bool {
    let expected = normalize_dir(expected_dir);
    match extract_config_dir(cmdline) {
        Some(actual) => normalize_dir(Path::new(&actual)) == expected,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_process_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = LoreProcessManager::new(temp_dir.path());

        assert_eq!(
            manager.config_path(),
            temp_dir
                .path()
                .join("lore")
                .join("config")
                .join("local.toml")
        );
        assert_eq!(
            manager.log_path(),
            temp_dir
                .path()
                .join("lore")
                .join("logs")
                .join("loreserver.log")
        );
    }

    #[test]
    fn test_is_running() {
        // Test with current process PID
        let current_pid = std::process::id();
        assert!(LoreProcessManager::is_running(current_pid));

        // Test with invalid PID
        assert!(!LoreProcessManager::is_running(999999));
    }

    #[test]
    fn test_expected_config_dir() {
        let temp_dir = TempDir::new().unwrap();
        let manager = LoreProcessManager::new(temp_dir.path());
        assert_eq!(
            manager.expected_config_dir(),
            temp_dir.path().join("lore").join("config")
        );
    }

    #[test]
    fn test_extract_config_dir() {
        assert_eq!(
            extract_config_dir("loreserver --config /home/u/.px/lore/config"),
            Some("/home/u/.px/lore/config".to_string())
        );
        assert_eq!(
            extract_config_dir("loreserver --config=/tmp/t/lore/config --verbose"),
            Some("/tmp/t/lore/config".to_string())
        );
        assert_eq!(
            extract_config_dir("loreserver --config /x/lore/config/"),
            Some("/x/lore/config".to_string())
        );
        assert_eq!(extract_config_dir("loreserver --verbose"), None);
        assert_eq!(extract_config_dir("other --config"), None);
    }

    #[test]
    fn test_cmdline_has_config_dir() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().join("lore").join("config");
        std::fs::create_dir_all(&config_dir).unwrap();

        let cmdline = format!("loreserver --config {}", config_dir.display());
        assert!(cmdline_has_config_dir(&cmdline, &config_dir));

        let foreign = format!("loreserver --config {}", temp_dir.path().join("other").display());
        assert!(!cmdline_has_config_dir(&foreign, &config_dir));

        assert!(!cmdline_has_config_dir("loreserver", &config_dir));
    }

    #[test]
    fn test_cmdline_has_config_dir_matches_equivalent_paths() {
        // Canonicalization must unify symlinked spellings of the same dir.
        let temp_dir = TempDir::new().unwrap();
        let real_dir = temp_dir.path().join("realcfg");
        std::fs::create_dir_all(&real_dir).unwrap();
        let link_dir = temp_dir.path().join("linkcfg");
        std::os::unix::fs::symlink(&real_dir, &link_dir).unwrap();

        let cmdline = format!("loreserver --config {}", link_dir.display());
        assert!(cmdline_has_config_dir(&cmdline, &real_dir));
    }

    #[test]
    #[cfg(unix)]
    fn test_process_cmdline_reads_self() {
        let cmdline = process_cmdline(std::process::id()).expect("readable self cmdline");
        assert!(!cmdline.is_empty());
    }
}
