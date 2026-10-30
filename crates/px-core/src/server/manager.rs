// SPDX-FileCopyrightText: 2026 Digital Creations
// SPDX-License-Identifier: MIT
//! Server manager for Lore repository backend
//!
//! Provides complete lifecycle management for the Local provider:
//! - ensureInstalled()
//! - ensureConfigured()
//! - ensureRunning()
//! - start()
//! - stop()
//! - restart()
//! - status()
//! - healthCheck()
//! - upgrade()

use anyhow::{Context, Result};
use std::path::Path;
use std::time::Duration;
use tracing::{error, info, warn};

use super::{
    cert::generate_certificates, config::generate_local_config, lock::ProcessLock,
    process::LoreProcessManager, version::verify_lore_installation,
};

use crate::LoreInstaller;
use crate::server::error_ids;

/// Server manager for Lore repository backend
pub struct ServerManager {
    px_home: std::path::PathBuf,
    http_port: u16,
    health_check_timeout: Duration,
    startup_timeout: Duration,
    retry_interval: Duration,
    max_retries: usize,
}

/// Outcome of verifying which process owns the Lore health-port listener.
enum ListenerCheck {
    /// The listener is this PX home's loreserver — safe to adopt.
    Ours,
    /// A foreign loreserver was stopped so ours can start.
    ReplacedForeignLore,
}

impl ServerManager {
    /// Create a new server manager
    pub fn new(px_home: &Path) -> Self {
        Self {
            px_home: px_home.to_path_buf(),
            http_port: 41339, // Default Lore HTTP port
            health_check_timeout: Duration::from_secs(5),
            startup_timeout: Duration::from_secs(10),
            retry_interval: Duration::from_secs(1),
            max_retries: 3,
        }
    }

    /// Set custom HTTP port
    pub fn with_http_port(mut self, port: u16) -> Self {
        self.http_port = port;
        self
    }

    /// Ensure Lore is installed
    pub fn ensure_installed(&self) -> Result<()> {
        info!("Checking Lore installation");

        let status =
            verify_lore_installation().context("Failed to verify Lore installation status")?;

        if !status.is_fully_compatible() {
            let message = status.status_message();

            // If completely missing, try to install automatically
            if !status.cli_installed || !status.server_installed {
                info!("Lore not detected, attempting automatic installation");
                let installer = LoreInstaller::new(None);
                installer.install_all().context(format!(
                    "[{}] Failed to automatically install Lore",
                    error_ids::ERR_LORE_INSTALL_FAILED
                ))?;

                // Re-verify after install
                let new_status = verify_lore_installation()?;
                if !new_status.is_fully_compatible() {
                    anyhow::bail!(
                        "[{}] Lore installation failed or is incompatible: {}. \
                         Fix: run 'px install lore' to manually install.",
                        error_ids::ERR_LORE_INCOMPATIBLE,
                        new_status.status_message()
                    );
                }
                return Ok(());
            }

            // Incompatible but present
            error!(
                installed = status.cli_installed,
                cli_version = status.cli_version.as_ref().map(|v| v.raw.as_str()).unwrap_or("not detected"),
                server_installed = status.server_installed,
                server_version = status.server_version.as_ref().map(|v| v.raw.as_str()).unwrap_or("not detected"),
                pinned = %status.pinned_version,
                "Lore installation incompatible: {}",
                message
            );
            anyhow::bail!(
                "[{}] Lore installation is incompatible: {}. \
                 Required version: {}. \
                 Fix: run 'px install lore' to install the correct version.",
                error_ids::ERR_LORE_INCOMPATIBLE,
                message,
                status.pinned_version
            );
        }

        info!(
            pinned_version = %status.pinned_version,
            "Lore installation is compatible"
        );
        Ok(())
    }

    /// Ensure Lore is configured
    pub fn ensure_configured(&self) -> Result<()> {
        info!(px_home = %self.px_home.display(), "Ensuring Lore configuration");

        // Generate configuration
        let config_files = generate_local_config(&self.px_home).context(format!(
            "[{}] Failed to generate Lore configuration at '{}'. \
                 Check directory permissions and disk space.",
            error_ids::ERR_LORE_CONFIG_FAILED,
            self.px_home.display()
        ))?;
        tracing::debug!(config_path = %config_files.config_path.display(), "Lore config generated");

        // Generate certificates
        let cert_dir = self.px_home.join("lore").join("certs");
        let cert_files = generate_certificates(&cert_dir).context(format!(
            "[{}] Failed to generate Lore certificates at '{}'. \
                 Check directory permissions.",
            error_ids::ERR_LORE_CERT_FAILED,
            cert_dir.display()
        ))?;
        tracing::debug!(
            cert = %cert_files.cert_path.display(),
            key = %cert_files.key_path.display(),
            "Lore certificates generated"
        );

        info!("Lore configuration and certificates are ready");
        Ok(())
    }

    /// Ensure Lore server is running
    pub async fn ensure_running(&self) -> Result<()> {
        info!("Ensuring Lore server is running");

        // Try to acquire lock
        let mut lock = ProcessLock::new(&self.px_home);
        if !lock.try_acquire()? {
            // Server lock is held - check if daemon is actually healthy
            let daemon_pid = lock
                .read_daemon_pid()
                .unwrap_or(None)
                .map(|p| p.to_string())
                .unwrap_or_else(|| "unknown".to_string());

            info!(
                daemon_pid = %daemon_pid,
                "Server lock already held, verifying daemon health"
            );

            if LoreProcessManager::health_check(self.http_port, self.health_check_timeout).await? {
                // Health is port-based — a foreign loreserver (stale test
                // instance, different PX home) can satisfy it while serving a
                // different data directory. Verify the listener was started
                // with this PX home's --config before adopting it.
                match self.verify_listener_identity().await? {
                    ListenerCheck::Ours => {
                        info!(daemon_pid = %daemon_pid, "Existing daemon is healthy and running");
                        return Ok(());
                    }
                    ListenerCheck::ReplacedForeignLore => {
                        info!("Foreign loreserver removed; starting this PX home's server");
                    }
                }
            }

            // Daemon is not healthy - the lock is stale (process crashed)
            // ProcessLock::try_acquire() already handles stale lock cleanup
            // but it returned false because the lock was held. We need to
            // force it to clean up the stale lock and retry.
            warn!(
                daemon_pid = %daemon_pid,
                "Server lock held but daemon health check failed — \
                 the daemon may have crashed. Attempting automatic recovery."
            );

            // Force stale lock cleanup by creating a new lock instance
            // ProcessLock::try_acquire() will detect the stale PID and clean it up
            let mut recovery_lock = ProcessLock::new(&self.px_home);
            if !recovery_lock.try_acquire()? {
                anyhow::bail!(
                    "Failed to acquire lock after daemon crash. \
                     Another process may have started the server. \
                     Run 'px doctor' or manually remove '{}' to force restart.",
                    self.px_home.join("lore").join("pid").display()
                );
            }

            // Validate data directories exist before restart
            self.validate_data_directories()?;

            // Start server with the recovered lock
            self.start_internal(&mut recovery_lock).await?;

            info!("Lore server restarted successfully after crash recovery");
            return Ok(());
        }

        // Start server
        self.start_internal(&mut lock).await?;

        info!("Lore server is running and healthy");
        Ok(())
    }

    /// Verify the loreserver listening on the health port belongs to this
    /// PX home (`--config <px_home>/lore/config`).
    ///
    /// - Ours → return.
    /// - Foreign loreserver (stale test instance or another PX home) →
    ///   SIGTERM it so this PX home's server can bind the port. Lore ports are
    ///   a per-machine singleton; a foreign loreserver there is definitionally
    ///   squatting. Only `loreserver` processes are killed — never unknown
    ///   processes.
    /// - Non-lore process or unreadable cmdline → bail with actionable error.
    async fn verify_listener_identity(&self) -> Result<ListenerCheck> {
        use crate::server::process::ListenerIdentity;

        let process_manager = LoreProcessManager::new(&self.px_home);
        match process_manager.listener_identity(self.http_port) {
            ListenerIdentity::Ours(pid) => {
                tracing::debug!(pid, "Port listener verified as this PX home's loreserver");
                Ok(ListenerCheck::Ours)
            }
            ListenerIdentity::ForeignLore {
                pid,
                cmdline,
                config_dir,
            } => {
                warn!(
                    pid,
                    config_dir = %config_dir,
                    expected_dir = %process_manager.expected_config_dir().display(),
                    cmdline = %cmdline,
                    "Foreign loreserver occupies the port — stopping it so this \
                     PX home's server can start with the correct data directory"
                );
                LoreProcessManager::stop(pid).with_context(|| {
                    format!(
                        "Foreign Lore server (PID {pid}) is running with config '{config_dir}' \
                         on port {}, but this PX home requires '{}'. Could not stop it \
                         automatically — run `kill {pid}` and retry.",
                        self.http_port,
                        process_manager.expected_config_dir().display()
                    )
                })?;
                Ok(ListenerCheck::ReplacedForeignLore)
            }
            ListenerIdentity::ForeignProcess { pid, cmdline } => {
                anyhow::bail!(
                    "Another process (PID {pid}) is listening on Lore health port {} \
                     and is not a loreserver: {cmdline}. Stop it or change the port \
                     before PX can start the Lore server.",
                    self.http_port
                )
            }
            ListenerIdentity::Unknown => {
                // Cannot verify — log loudly but do not block. The PID file
                // still tracks our daemon; unverifiable is not proof foreign.
                warn!(
                    port = self.http_port,
                    "Could not verify the identity of the Lore server on the port; \
                     proceeding but the server may belong to another data directory"
                );
                Ok(ListenerCheck::Ours)
            }
            ListenerIdentity::NoListener => {
                // Health answered but lsof shows nothing — inconsistent view.
                warn!(
                    port = self.http_port,
                    "Health check passed but no TCP listener found; restarting server"
                );
                Ok(ListenerCheck::ReplacedForeignLore)
            }
        }
    }

    /// Validate that data directories exist and are accessible
    fn validate_data_directories(&self) -> Result<()> {
        let immutable_dir = self.px_home.join("lore").join("store").join("immutable");
        let mutable_dir = self.px_home.join("lore").join("store").join("mutable");

        if !immutable_dir.exists() {
            warn!(
                "Immutable store directory missing at '{}'. Server will initialize with empty store.",
                immutable_dir.display()
            );
        }

        if !mutable_dir.exists() {
            warn!(
                "Mutable store directory missing at '{}'. Server will initialize with empty store.",
                mutable_dir.display()
            );
        }

        // Verify config file exists and is readable
        let config_path = self.px_home.join("lore").join("config").join("local.toml");
        if !config_path.exists() {
            anyhow::bail!(
                "Lore configuration missing at '{}'. Run 'px init' to regenerate.",
                config_path.display()
            );
        }

        // Try to parse config to verify it's not corrupted
        let config_content = std::fs::read_to_string(&config_path).with_context(|| {
            format!(
                "Failed to read Lore configuration at '{}'",
                config_path.display()
            )
        })?;

        if toml::from_str::<toml::Value>(&config_content).is_err() {
            anyhow::bail!(
                "Lore configuration at '{}' is corrupted. Run 'px init --reset' to regenerate.",
                config_path.display()
            );
        }

        Ok(())
    }

    /// Start Lore server
    pub async fn start(&self) -> Result<()> {
        info!("Starting Lore server");

        let mut lock = ProcessLock::new(&self.px_home);
        if !lock.try_acquire()? {
            anyhow::bail!("Server is already running (lock held)");
        }

        self.start_internal(&mut lock).await?;

        info!("Lore server started successfully");
        Ok(())
    }

    /// Internal start implementation
    async fn start_internal(&self, lock: &mut ProcessLock) -> Result<()> {
        let process_manager = LoreProcessManager::new(&self.px_home);

        // Start the process in detached mode
        let daemon_pid = process_manager
            .start()
            .context("Failed to start Lore server process")?;

        // Write the actual daemon PID to the lock file
        lock.write_daemon_pid(daemon_pid)
            .context("Failed to write server PID to lock file")?;

        tracing::info!(
            daemon_pid,
            "Lore daemon spawned in detached mode, waiting for health check"
        );

        // Validate the daemon process is still running after write
        // If it crashed immediately, we have a stale lock
        if !LoreProcessManager::is_running(daemon_pid) {
            tracing::error!(
                daemon_pid,
                "Lore daemon crashed immediately after spawn, cleaning up stale lock"
            );
            lock.release()?;
            anyhow::bail!(
                "Lore server process (PID {}) crashed immediately after startup. \
                 Check logs at '{}' for errors.",
                daemon_pid,
                self.px_home
                    .join("lore")
                    .join("logs")
                    .join("loreserver.log")
                    .display()
            );
        }

        // Wait for server to become healthy
        let mut retries = 0;
        while retries < self.max_retries {
            match crate::server::process::LoreProcessManager::wait_for_healthy(
                self.http_port,
                self.startup_timeout,
                self.retry_interval,
            )
            .await
            {
                Ok(_) => {
                    // The health check is port-based — verify the responder is
                    // actually OUR daemon (a foreign loreserver can answer while
                    // our daemon failed to bind the port).
                    match self.verify_listener_identity().await {
                        Ok(ListenerCheck::Ours) => return Ok(()),
                        Ok(ListenerCheck::ReplacedForeignLore) => {
                            // Foreign server was evicted; ours may need another
                            // retry cycle to bind and become healthy.
                            retries += 1;
                            if retries < self.max_retries {
                                tokio::time::sleep(self.retry_interval).await;
                            }
                            continue;
                        }
                        Err(e) => {
                            let _ = lock.release();
                            return Err(e);
                        }
                    }
                }
                Err(e) => {
                    retries += 1;
                    warn!(
                        attempt = retries,
                        max_retries = self.max_retries,
                        error = %e,
                        "Health check attempt {} of {} failed",
                        retries, self.max_retries
                    );
                    if retries < self.max_retries {
                        tokio::time::sleep(self.retry_interval).await;
                    }
                }
            }
        }

        // Release lock on failure
        lock.release()?;
        anyhow::bail!(
            "[{}] Lore server failed to become healthy after {} retries. \
             Check logs at '{}' for startup errors.",
            error_ids::ERR_LORE_STARTUP_FAILED,
            self.max_retries,
            self.px_home
                .join("lore")
                .join("logs")
                .join("loreserver.log")
                .display()
        );
    }

    /// Stop Lore server
    pub fn stop(&self) -> Result<()> {
        let lock_file = self.px_home.join("lore").join("pid");
        if !lock_file.exists() {
            info!(
                "No lock file found at '{}', server may not be running",
                lock_file.display()
            );
            return Ok(());
        }

        let pid_str = std::fs::read_to_string(&lock_file).context(format!(
            "Failed to read PID lock file at '{}'",
            lock_file.display()
        ))?;
        let pid: u32 = pid_str.trim().parse().context(format!(
            "Failed to parse PID '{}' from lock file at '{}'",
            pid_str.trim(),
            lock_file.display()
        ))?;

        info!(pid, "Stopping Lore server process");

        LoreProcessManager::stop(pid).context(format!(
            "Failed to stop Lore server process (PID {}). \
                 The process may have already exited.",
            pid
        ))?;

        // Remove lock file
        std::fs::remove_file(&lock_file).context(format!(
            "Failed to remove lock file at '{}'",
            lock_file.display()
        ))?;

        info!(pid, "Lore server stopped successfully");
        Ok(())
    }

    /// Restart Lore server
    pub async fn restart(&self) -> Result<()> {
        info!("Restarting Lore server");

        self.stop()?;
        tokio::time::sleep(Duration::from_secs(2)).await; // Give it time to stop
        self.start().await?;

        info!("Lore server restarted");
        Ok(())
    }

    /// Get server status
    pub async fn status(&self) -> Result<ServerStatus> {
        let lock_file = self.px_home.join("lore").join("pid");

        // Prioritize health check over PID tracking since the server may be re-parented
        let healthy =
            LoreProcessManager::health_check(self.http_port, self.health_check_timeout).await?;

        // Only check PID if health check fails, to provide better error messages
        let running = if healthy {
            true
        } else if lock_file.exists() {
            let pid_str =
                std::fs::read_to_string(&lock_file).context("Failed to read lock file")?;
            let pid: u32 = pid_str
                .trim()
                .parse()
                .context("Failed to parse PID from lock file")?;

            LoreProcessManager::is_running(pid)
        } else {
            false
        };

        let configured = self.px_home.join("lore").join("config").exists();

        Ok(ServerStatus {
            running,
            healthy,
            configured,
            http_port: self.http_port,
        })
    }

    /// Perform health check
    pub async fn health_check(&self) -> Result<bool> {
        LoreProcessManager::health_check(self.http_port, self.health_check_timeout).await
    }

    /// Upgrade Lore (not implemented yet)
    pub fn upgrade(&self) -> Result<()> {
        anyhow::bail!("Lore upgrade is not yet implemented");
    }
}

/// Server status information
#[derive(Debug, Clone)]
pub struct ServerStatus {
    pub running: bool,
    pub healthy: bool,
    pub configured: bool,
    pub http_port: u16,
}

impl ServerStatus {
    /// Check if server is ready for use
    pub fn is_ready(&self) -> bool {
        self.running && self.healthy && self.configured
    }

    /// Get a human-readable status message
    pub fn status_message(&self) -> String {
        if self.is_ready() {
            "Server is running and healthy".to_string()
        } else if !self.configured {
            "Server is not configured".to_string()
        } else if !self.running {
            "Server is not running".to_string()
        } else {
            "Server is running but not healthy".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use tempfile::TempDir;

    #[test]
    fn test_server_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ServerManager::new(temp_dir.path());
        assert_eq!(manager.http_port, 41339);
    }

    #[test]
    fn test_concurrent_lock_acquisition() {
        let temp_dir = TempDir::new().unwrap();
        let px_home = Arc::new(temp_dir.path().to_path_buf());

        // Create lock directory first
        std::fs::create_dir_all(px_home.join("lore")).unwrap();

        // First thread acquires lock
        let px_home_clone1 = Arc::clone(&px_home);
        let handle1 = thread::spawn(move || {
            let mut lock = ProcessLock::new(&px_home_clone1);
            lock.try_acquire().unwrap();
            // Hold the lock for a moment
            thread::sleep(std::time::Duration::from_millis(100));
            true
        });

        // Second thread tries to acquire while first holds it
        let px_home_clone2 = Arc::clone(&px_home);
        let handle2 = thread::spawn(move || {
            thread::sleep(std::time::Duration::from_millis(10)); // Ensure first thread runs first
            let mut lock = ProcessLock::new(&px_home_clone2);
            lock.try_acquire().unwrap_or(false)
        });

        handle1.join().unwrap();
        let acquired = handle2.join().unwrap();

        // Second thread should not acquire the lock (first still holds it)
        assert!(
            !acquired,
            "Second thread should not acquire lock while first holds it"
        );
    }

    #[test]
    fn test_server_manager_custom_port() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ServerManager::new(temp_dir.path()).with_http_port(8080);
        assert_eq!(manager.http_port, 8080);
    }

    #[test]
    fn test_server_status_message() {
        let status = ServerStatus {
            running: true,
            healthy: true,
            configured: true,
            http_port: 41339,
        };
        assert_eq!(status.status_message(), "Server is running and healthy");

        let status = ServerStatus {
            running: false,
            healthy: false,
            configured: false,
            http_port: 41339,
        };
        assert_eq!(status.status_message(), "Server is not configured");
    }

    #[test]
    fn test_data_directory_consistency() {
        let temp_dir = TempDir::new().unwrap();
        let px_home = temp_dir.path();

        // Create server manager
        let manager = ServerManager::new(px_home);

        // Ensure configuration is generated
        manager.ensure_configured().unwrap();

        // Verify data directory paths are derived from px_home
        let expected_config_dir = px_home.join("lore").join("config");
        let _expected_immutable_dir = px_home.join("lore").join("store").join("immutable");
        let _expected_mutable_dir = px_home.join("lore").join("store").join("mutable");

        assert!(
            expected_config_dir.exists(),
            "Config directory should exist"
        );
        assert!(
            expected_config_dir.join("local.toml").exists(),
            "local.toml should exist"
        );

        // Read the config to verify store paths
        let config_content =
            std::fs::read_to_string(expected_config_dir.join("local.toml")).unwrap();
        assert!(
            config_content.contains(
                &px_home
                    .join("lore")
                    .join("store")
                    .join("immutable")
                    .to_string_lossy()
                    .replace('\\', "\\\\")
            )
        );
        assert!(
            config_content.contains(
                &px_home
                    .join("lore")
                    .join("store")
                    .join("mutable")
                    .to_string_lossy()
                    .replace('\\', "\\\\")
            )
        );

        // Create another manager with the same px_home
        let _manager2 = ServerManager::new(px_home);
        _manager2.ensure_configured().unwrap();

        // Verify it uses the same config (not regenerated)
        let config_content2 =
            std::fs::read_to_string(expected_config_dir.join("local.toml")).unwrap();
        assert_eq!(
            config_content, config_content2,
            "Config should not be regenerated on second call"
        );
    }

    #[test]
    fn test_process_manager_uses_same_config_path() {
        let temp_dir = TempDir::new().unwrap();
        let px_home = temp_dir.path();

        // Create process manager
        let process_manager = LoreProcessManager::new(px_home);

        // Verify config path is derived from px_home
        let expected_config_path = px_home.join("lore").join("config").join("local.toml");
        assert_eq!(process_manager.config_path(), &expected_config_path);

        // Verify log path is derived from px_home
        let expected_log_path = px_home.join("lore").join("logs").join("loreserver.log");
        assert_eq!(process_manager.log_path(), &expected_log_path);

        // Create another process manager with the same px_home
        let process_manager2 = LoreProcessManager::new(px_home);
        assert_eq!(process_manager2.config_path(), &expected_config_path);
        assert_eq!(process_manager2.log_path(), &expected_log_path);
    }

    #[test]
    fn test_lock_file_uses_px_home() {
        let temp_dir = TempDir::new().unwrap();
        let px_home = temp_dir.path();

        // Create lock
        let mut lock = ProcessLock::new(px_home);

        // Verify lock file path is derived from px_home by checking if it can be acquired
        let acquired = lock.try_acquire().unwrap();
        assert!(acquired, "Lock should be acquirable");

        // Verify lock file exists at expected location
        let expected_lock_path = px_home.join("lore").join("pid");
        assert!(
            expected_lock_path.exists(),
            "Lock file should exist at expected path"
        );

        // Create another lock with the same px_home
        let mut lock2 = ProcessLock::new(px_home);
        let acquired2 = lock2.try_acquire().unwrap();
        assert!(
            !acquired2,
            "Second lock should not be acquirable (already held)"
        );
    }

    #[test]
    fn test_server_manager_preserves_px_home() {
        let temp_dir = TempDir::new().unwrap();
        let px_home = temp_dir.path();

        let manager = ServerManager::new(px_home);
        assert_eq!(manager.px_home, px_home);

        // Verify all operations use the same px_home
        let mut lock = ProcessLock::new(&manager.px_home);
        let expected_lock_path = px_home.join("lore").join("pid");
        let acquired = lock.try_acquire().unwrap();
        assert!(acquired, "Lock should be acquirable");
        assert!(
            expected_lock_path.exists(),
            "Lock file should exist at expected path"
        );

        let process_manager = LoreProcessManager::new(&manager.px_home);
        assert_eq!(
            process_manager.config_path(),
            px_home.join("lore").join("config").join("local.toml")
        );
    }

    #[test]
    fn test_stale_lock_cleanup_preserves_data_directory() {
        let temp_dir = TempDir::new().unwrap();
        let px_home = temp_dir.path();

        let manager = ServerManager::new(px_home);

        // Ensure configuration is generated
        manager.ensure_configured().unwrap();

        // Create data directories to simulate existing data
        let immutable_dir = px_home.join("lore").join("store").join("immutable");
        let mutable_dir = px_home.join("lore").join("store").join("mutable");
        std::fs::create_dir_all(&immutable_dir).unwrap();
        std::fs::create_dir_all(&mutable_dir).unwrap();

        // Create some test data
        std::fs::write(immutable_dir.join("test.dat"), "test data").unwrap();
        std::fs::write(mutable_dir.join("test2.dat"), "test data 2").unwrap();

        // Create a stale lock file with a non-existent PID
        let lock_file = px_home.join("lore").join("pid");
        std::fs::create_dir_all(lock_file.parent().unwrap()).unwrap();
        std::fs::write(&lock_file, "9999999").unwrap();

        // Create a new manager - it should clean up the stale lock
        let _manager2 = ServerManager::new(px_home);
        let mut lock = ProcessLock::new(px_home);
        // The stale lock should be cleaned up during try_acquire
        let acquired = lock.try_acquire().unwrap();
        assert!(acquired, "Should acquire lock after stale lock cleanup");

        // Verify data directories still exist and contain data
        assert!(
            immutable_dir.exists(),
            "Immutable store directory should persist"
        );
        assert!(
            mutable_dir.exists(),
            "Mutable store directory should persist"
        );
        assert_eq!(
            std::fs::read_to_string(immutable_dir.join("test.dat")).unwrap(),
            "test data",
            "Immutable store data should persist"
        );
        assert_eq!(
            std::fs::read_to_string(mutable_dir.join("test2.dat")).unwrap(),
            "test data 2",
            "Mutable store data should persist"
        );

        // Verify config is still using the same data directories
        let config_content =
            std::fs::read_to_string(px_home.join("lore").join("config").join("local.toml"))
                .unwrap();
        assert!(
            config_content.contains(
                &px_home
                    .join("lore")
                    .join("store")
                    .join("immutable")
                    .to_string_lossy()
                    .replace('\\', "\\\\")
            )
        );
        assert!(
            config_content.contains(
                &px_home
                    .join("lore")
                    .join("store")
                    .join("mutable")
                    .to_string_lossy()
                    .replace('\\', "\\\\")
            )
        );
    }

    #[test]
    fn test_validate_data_directories() {
        let temp_dir = TempDir::new().unwrap();
        let px_home = temp_dir.path();

        let manager = ServerManager::new(px_home);

        // Ensure configuration is generated
        manager.ensure_configured().unwrap();

        // Validation should succeed with config present
        assert!(manager.validate_data_directories().is_ok());

        // Corrupt the config file
        let config_path = px_home.join("lore").join("config").join("local.toml");
        std::fs::write(&config_path, "corrupted [[[ invalid").unwrap();

        // Validation should fail with corrupted config
        assert!(manager.validate_data_directories().is_err());

        // Remove corrupted config and regenerate
        std::fs::remove_file(&config_path).unwrap();
        manager.ensure_configured().unwrap();

        // Validation should succeed again
        assert!(manager.validate_data_directories().is_ok());

        // Remove config file
        std::fs::remove_file(&config_path).unwrap();

        // Validation should fail with missing config
        assert!(manager.validate_data_directories().is_err());
    }
}
