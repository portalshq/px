//! Error types for the PX protocol.
//!
//! All PX errors are surfaced through [`PxError`], which captures the exact
//! failure domain (URI parsing, manifest I/O, VCS operations, resolution, etc.)
//! with enough context for callers to produce actionable diagnostics.

use thiserror::Error;

/// Top-level error type for all PX operations.
#[derive(Error, Debug)]
pub enum PxError {
    // ── URI Errors ──────────────────────────────────────────────────────
    #[error("invalid PX URI '{uri}': {reason}")]
    InvalidUri { uri: String, reason: String },

    // ── Manifest Errors ─────────────────────────────────────────────────
    #[error("manifest not found: {0}")]
    ManifestNotFound(String),

    #[error("manifest parse error for '{path}': {source}")]
    ManifestParseError {
        path: String,
        source: serde_yaml::Error,
    },

    #[error("manifest validation error: {0}")]
    ManifestValidationError(String),

    #[error("manifest write error for '{path}': {source}")]
    ManifestWriteError {
        path: String,
        source: std::io::Error,
    },

    // ── Query Errors ────────────────────────────────────────────────────
    #[error("query path not found: '{path}' in manifest '{manifest_id}'")]
    QueryPathNotFound { path: String, manifest_id: String },

    #[error("invalid query path: '{0}'")]
    InvalidQueryPath(String),

    // ── Repository Errors ───────────────────────────────────────────────
    #[error("repository not found at '{0}'")]
    RepositoryNotFound(String),

    #[error(
        "operation requires a local PX working tree for '{repository}'; use 'px pull {repository}' first"
    )]
    LocalWorkingTreeRequired { repository: String },

    #[error("repository already exists at '{0}'")]
    RepositoryAlreadyExists(String),

    // ── Entity Type Errors ─────────────────────────────────────────────
    #[error("entity type not found: '{0}'")]
    EntityTypeNotFound(String),

    // ── VCS Errors ──────────────────────────────────────────────────────
    /// Concise display with original diagnostics retained for verbose CLI output.
    #[error("{message}")]
    LoreFailure { message: String, details: String },

    #[error("VCS error: {0}")]
    VcsError(String),

    #[error("ref not found: '{0}'")]
    RefNotFound(String),

    // ── Version-control backend errors ──────────────────────────────────
    #[error(
        "cannot {operation}: no version-control backend is configured. \
         Filesystem state is preserved, but history/sync is unavailable. \
         Configure one with 'px configure'."
    )]
    BackendNotConfigured { operation: String },

    #[error("version-control backend is unavailable: {message}")]
    BackendUnavailable { message: String },

    #[error("version-control backend operation failed ({operation}): {message}")]
    BackendOperationFailed { operation: String, message: String },

    // ── Content Addressing Errors ───────────────────────────────────────
    #[error("content hash mismatch: expected {expected}, got {actual}")]
    ContentHashMismatch { expected: String, actual: String },

    // ── I/O ─────────────────────────────────────────────────────────────
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    // ── Merge Errors ────────────────────────────────────────────────────
    #[error("merge conflict at '{path}': {details}")]
    MergeConflict { path: String, details: String },

    #[error("SDL parse error: {0}")]
    SdlParseError(String),

    #[error("SDL validation error: {reason}")]
    SdlValidationError { reason: String },

    #[error("merge strategy error at '{path}': {reason}")]
    MergeStrategyError { path: String, reason: String },

    #[error("merge validation error: {reason}")]
    MergeValidationError { reason: String },

    // ── Resolution Errors ───────────────────────────────────────────────
    #[error("no branch or commit specified and no default_branch configured. ")]
    NoDefaultBranch,

    #[error("unable to resolve resource '{address}': {message}")]
    ResolutionFailed { address: String, message: String },

    // ── Permission ──────────────────────────────────────────────────────
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    // ── gRPC ─────────────────────────────────────────────────────────────
    #[error("gRPC error: {0}")]
    GrpcError(String),

    // ── Catch-all ───────────────────────────────────────────────────────
    #[error("{0}")]
    Other(String),
}

/// Result type alias using [`PxError`].
pub type PxResult<T> = Result<T, PxError>;

/// Extract the useful cause from Lore's nested error/stack output. Keep the
/// original separately on LoreFailure; presentation never destroys diagnostics.
pub fn clean_lore_error(stderr: &str, exit_code: i32) -> String {
    // JSON-mode Lore writes failures to stdout as complete/error events.
    // Extract only the public message; traceLocations remain in verbose details.
    let events: Vec<serde_json::Value> = stderr
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    let structured = events
        .iter()
        .find_map(|event| {
            event
                .get("data")
                .and_then(|data| data.get("error"))
                .and_then(|error| error.get("message"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .or_else(|| {
            events.iter().rev().find_map(|event| {
                let data = event.get("data")?;
                let level = data
                    .get("level")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                if level.eq_ignore_ascii_case("error")
                    || event.get("tagName").and_then(serde_json::Value::as_str) == Some("error")
                {
                    data.get("message")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned)
                } else {
                    None
                }
            })
        });
    let stderr = structured.as_deref().unwrap_or(stderr);
    let lower = stderr.to_ascii_lowercase();
    if [
        "not authenticated",
        "authentication required",
        "unauthenticated",
        "not authorized",
        "permission denied",
    ]
    .iter()
    .any(|s| lower.contains(s))
    {
        return "Lore authentication or permission failed; run `px auth login` and check repository access".into();
    }
    if [
        "transport error",
        "connection refused",
        "failed to connect",
        "connection timed out",
        "dns error",
        "acquiring remote",
    ]
    .iter()
    .any(|s| lower.contains(s))
    {
        return "Could not connect to the repository server; check its remote URL, network connection, and `px doctor`".into();
    }
    if (lower.contains("content") || lower.contains("address") || lower.contains("fragment"))
        && ["not found", "missing", "does not exist"]
            .iter()
            .any(|s| lower.contains(s))
    {
        return "Repository content is unavailable on the server; commit and push the source files, then retry `px pull`. Use `px doctor` if content is still missing".into();
    }
    if lower.contains("not a lore workspace") || lower.contains("not an initialised lore workspace")
    {
        return "A local Lore checkout is required; run `px pull <repository>` first".into();
    }
    let message = stderr
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let line = line
                .strip_prefix("Error:")
                .or_else(|| line.strip_prefix("error:"))
                .unwrap_or(line)
                .trim();
            if line.is_empty()
                || serde_json::from_str::<serde_json::Value>(line).is_ok()
                || line == "Caused by:"
                || line.starts_with("Stack backtrace:")
                || line.starts_with("at ")
                || line.contains(".rs:")
                || line.starts_with('/')
                || line.starts_with("note:")
                || line
                    .split_once(':')
                    .is_some_and(|(prefix, _)| prefix.trim().parse::<usize>().is_ok())
            {
                None
            } else {
                Some(line)
            }
        })
        .next();
    match message {
        Some(message) => {
            format!("Lore operation failed: {message}. Retry with --verbose for details")
        }
        None => {
            format!("Lore operation failed (exit {exit_code}); retry with --verbose for details")
        }
    }
}

impl PxError {
    /// Attach operation context without losing Lore's original verbose details.
    pub fn context(self, context: impl AsRef<str>) -> Self {
        match self {
            Self::LoreFailure { message, details } => Self::LoreFailure {
                message: format!("{}: {message}", context.as_ref()),
                details,
            },
            other => Self::VcsError(format!("{}: {other}", context.as_ref())),
        }
    }
}

#[cfg(test)]
mod lore_error_tests {
    use super::*;
    #[test]
    fn json_errors_extract_the_cause_and_never_print_event_payloads() {
        let event = r#"{"tagName":"complete","data":{"error":{"message":"content address not found","traceLocations":["/private/internal.rs:42"]}}}"#;
        assert!(clean_lore_error(event, 1).contains("commit and push"));
        let log = r#"{"tagName":"log","data":{"level":"ERROR","message":"revision is locked","traceLocations":["/private/internal.rs:42"]}}"#;
        let clean = clean_lore_error(log, 1);
        assert!(clean.contains("revision is locked"));
        assert!(!clean.contains("traceLocations"));
        let unknown = r#"{"tagName":"diagnostic","data":{"internal":"/private/internal.rs:42"}}"#;
        assert_eq!(
            clean_lore_error(unknown, 7),
            "Lore operation failed (exit 7); retry with --verbose for details"
        );
    }

    #[test]
    fn lore_errors_are_actionable_without_internal_paths() {
        for (input, expected) in [
            (
                "Error: Unauthenticated\n at /build/lore/src/auth.rs:90",
                "px auth login",
            ),
            (
                "transport error\nStack backtrace:\n 0: /build/lore.rs:90",
                "remote URL",
            ),
            (
                "content address not found\n at /build/store.rs:90",
                "commit and push",
            ),
            (
                "Error: revision is locked\nStack backtrace:\n 0: lore::internal\n at /build/lore.rs:90",
                "revision is locked",
            ),
        ] {
            let clean = clean_lore_error(input, 1);
            assert!(clean.contains(expected), "{clean}");
            assert!(!clean.contains("/build"));
            assert!(!clean.contains("Stack backtrace"));
        }
    }
}
