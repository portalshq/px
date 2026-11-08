use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The JSON value passed to a protocol-v1 plugin on standard input.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Request {
    pub input: Value,
    pub context: Context,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entitlement: Option<Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Context {
    pub base_dir: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    /// Absolute host CLI path for delegated core mutations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub px_binary: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Response {
    Output { output: Value },
    Error { error: PluginError },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PluginError {
    pub code: String,
    pub message: String,
}

/// Execute one protocol call, with bounded output and a deadline.
pub async fn call(
    executable: &std::path::Path,
    tool: &str,
    request: &Request,
    timeout: std::time::Duration,
) -> Result<Response, String> {
    let mut command = tokio::process::Command::new(executable);
    command.args(["--px-plugin-protocol", "1", "call", tool]);
    run_command(command, request, timeout).await
}

/// Run a Python plugin inside the project's locked uv environment.
pub async fn call_uv(
    uv: &std::path::Path,
    project: &std::path::Path,
    entrypoint: &std::path::Path,
    tool: &str,
    request: &Request,
    timeout: std::time::Duration,
) -> Result<Response, String> {
    let mut command = tokio::process::Command::new(uv);
    command
        .arg("run")
        .arg("--locked")
        .args(["--managed-python", "--python", "3.13"])
        .arg("--project")
        .arg(project)
        .arg("python")
        .arg(entrypoint)
        .args(["--px-plugin-protocol", "1", "call", tool]);
    command.env(
        "UV_PYTHON_INSTALL_DIR",
        std::path::Path::new(&request.context.base_dir).join("tools/python"),
    );
    run_command(command, request, timeout).await
}

async fn run_command(
    mut command: tokio::process::Command,
    request: &Request,
    timeout: std::time::Duration,
) -> Result<Response, String> {
    use std::process::Stdio;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    const MAX: usize = 10 * 1024 * 1024;
    let input = serde_json::to_vec(request).map_err(|e| e.to_string())?;
    if input.len() > MAX {
        return Err("plugin input exceeds limit".into());
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut stdin = child.stdin.take().ok_or("plugin stdin unavailable")?;
    let stdout = child.stdout.take().ok_or("plugin stdout unavailable")?;
    let stderr = child.stderr.take().ok_or("plugin stderr unavailable")?;
    let run = async {
        let write = async {
            stdin.write_all(&input).await?;
            stdin.shutdown().await?;
            drop(stdin);
            Ok::<_, std::io::Error>(())
        };
        let read = async {
            let mut out = Vec::new();
            stdout.take((MAX + 1) as u64).read_to_end(&mut out).await?;
            Ok::<_, std::io::Error>(out)
        };
        let read_error = async {
            let mut out = Vec::new();
            stderr.take((MAX + 1) as u64).read_to_end(&mut out).await?;
            Ok::<_, std::io::Error>(out)
        };
        let ((), out, err, status) =
            tokio::try_join!(write, read, read_error, child.wait()).map_err(|e| e.to_string())?;
        if out.len() > MAX || err.len() > MAX {
            return Err("plugin output exceeds limit".into());
        }
        if !status.success() {
            return Err(format!(
                "plugin exited unsuccessfully: {}",
                String::from_utf8_lossy(&err)
            ));
        }
        let value: Value =
            serde_json::from_slice(&out).map_err(|e| format!("invalid plugin response: {e}"))?;
        let object = value
            .as_object()
            .ok_or("plugin response must be an object")?;
        if object.len() != 1 || !(object.contains_key("output") ^ object.contains_key("error")) {
            return Err("plugin response must contain exactly output or error".into());
        }
        serde_json::from_value(value).map_err(|e| format!("invalid plugin response: {e}"))
    };
    tokio::time::timeout(timeout, run)
        .await
        .map_err(|_| "plugin call timed out".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[cfg(unix)]
    #[tokio::test]
    async fn uv_runner_uses_locked_project_and_protocol_arguments() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let script = tmp.path().join("uv");
        let log = tmp.path().join("args");
        std::fs::write(&script, format!("#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\ncat >/dev/null\nprintf '{{\"output\":{{\"ok\":true}}}}'\n", log.display())).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let request = Request {
            input: json!({}),
            context: Context {
                base_dir: "/tmp".into(),
                repository: None,
                branch: None,
                session: None,
                px_binary: None,
            },
            entitlement: None,
        };
        let result = call_uv(
            &script,
            tmp.path(),
            &tmp.path().join("entry.py"),
            "tool",
            &request,
            std::time::Duration::from_secs(5),
        )
        .await
        .unwrap();
        assert!(matches!(result, Response::Output { .. }));
        let args = std::fs::read_to_string(log).unwrap();
        assert!(args.lines().any(|arg| arg == "--locked"));
        assert!(args.lines().any(|arg| arg == "--project"));
        assert!(args.lines().any(|arg| arg == "--px-plugin-protocol"));
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn calls_example_using_protocol_v1() {
        let executable = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/plugins/hello/bin/hello");
        let request = Request {
            input: json!({"name":"PX"}),
            context: Context {
                base_dir: "/tmp".into(),
                repository: None,
                branch: None,
                session: None,
                px_binary: None,
            },
            entitlement: None,
        };
        match call(
            &executable,
            "greet",
            &request,
            std::time::Duration::from_secs(5),
        )
        .await
        .unwrap()
        {
            Response::Output { output } => assert_eq!(output["greeting"], "Hello, PX!"),
            Response::Error { .. } => panic!("unexpected plugin error"),
        }
    }
}
