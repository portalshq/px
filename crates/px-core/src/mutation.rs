//! Manifest mutation primitives shared by the CLI and plugins.
//!
//! These helpers previously lived in the CLI binary. They are hoisted here so
//! that every caller (CLI commands, `px record`, built-in plugins) applies the
//! exact same path normalization, representation staging, and provenance
//! layout. All functions are synchronous and operate on an opened
//! [`Repository`]; committing is always the caller's explicit final step,
//! except for [`record_generation`], which performs one atomic commit.

use std::path::{Path, PathBuf};

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::{debug, info};

use crate::commit::{Change, Commit};
use crate::content::ContentHash;
use crate::error::PxError;
use crate::manifest::{Manifest, Representation};
use crate::repository::Repository;
use crate::uri::PxUri;

/// Top-level manifest sections that may be addressed explicitly by a key.
const EXPLICIT_SECTIONS: [&str; 5] = [
    "properties",
    "references",
    "representations",
    "provenance",
    "metadata",
];

/// Placeholder recorded when a generation tool did not return a model ID.
/// PX never invents model identifiers (see the provenance checklist).
pub const UNAVAILABLE: &str = "unavailable";

/// Prefix bare keys with `properties.`; keep explicitly sectioned keys.
pub fn normalized_manifest_path(key: &str) -> String {
    if key
        .split('.')
        .next()
        .is_some_and(|section| EXPLICIT_SECTIONS.contains(&section))
    {
        key.to_string()
    } else {
        format!("properties.{key}")
    }
}

/// Parse a CLI/MCP value as JSON when possible, otherwise keep it a string.
pub fn yaml_from_json_or_string(value: &str) -> serde_yaml::Value {
    serde_json::from_str::<Value>(value)
        .ok()
        .and_then(|value| serde_yaml::to_value(value).ok())
        .unwrap_or_else(|| serde_yaml::Value::String(value.to_string()))
}

fn path_parts(key: &str) -> Result<(String, Vec<String>), PxError> {
    let path = normalized_manifest_path(key);
    let parts: Vec<String> = path.split('.').map(str::to_string).collect();
    if parts.iter().any(String::is_empty) {
        return Err(PxError::InvalidQueryPath(format!(
            "empty path segment in '{key}'"
        )));
    }
    Ok((path, parts))
}

fn manifest_document(manifest: &Manifest) -> Result<Value, PxError> {
    serde_json::to_value(manifest).map_err(|e| PxError::ManifestValidationError(e.to_string()))
}

fn replace_manifest(manifest: &mut Manifest, document: Value) -> Result<(), PxError> {
    *manifest = serde_json::from_value(document)
        .map_err(|e| PxError::ManifestValidationError(format!("invalid manifest update: {e}")))?;
    Ok(())
}

/// Set `key` (dot path) on the manifest, creating intermediate objects.
pub fn set_manifest_value(
    manifest: &mut Manifest,
    key: &str,
    value: serde_yaml::Value,
) -> Result<(), PxError> {
    let (_, parts) = path_parts(key)?;
    let mut document = manifest_document(manifest)?;
    let mut current = document
        .as_object_mut()
        .ok_or_else(|| PxError::ManifestValidationError("manifest must be an object".into()))?;
    let (leaf, parents) = parts.split_last().expect("path has at least one segment");
    for part in parents {
        current = current
            .entry(part.clone())
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or_else(|| PxError::InvalidQueryPath(format!("'{part}' is not an object")))?;
    }
    let value =
        serde_json::to_value(value).map_err(|e| PxError::ManifestValidationError(e.to_string()))?;
    current.insert(leaf.clone(), value);
    replace_manifest(manifest, document)
}

/// Remove `key` (dot path) from the manifest; errors when the key is absent.
pub fn unset_manifest_value(manifest: &mut Manifest, key: &str) -> Result<(), PxError> {
    let (_, parts) = path_parts(key)?;
    let mut document = manifest_document(manifest)?;
    let mut current = document
        .as_object_mut()
        .ok_or_else(|| PxError::ManifestValidationError("manifest must be an object".into()))?;
    let (leaf, parents) = parts.split_last().expect("path has at least one segment");
    for part in parents {
        current = current
            .get_mut(part)
            .and_then(Value::as_object_mut)
            .ok_or_else(|| PxError::QueryPathNotFound {
                path: key.to_string(),
                manifest_id: String::new(),
            })?;
    }
    if current.remove(leaf).is_none() {
        return Err(PxError::QueryPathNotFound {
            path: key.to_string(),
            manifest_id: String::new(),
        });
    }
    replace_manifest(manifest, document)
}

/// Directory that holds an entity's representation files.
pub fn entity_dir(repo: &Repository, uri: &PxUri) -> PathBuf {
    repo.root
        .join(uri.entity_type.to_string())
        .join(&uri.entity_id)
}

/// Result of staging one representation file.
#[derive(Debug, Clone)]
pub enum StageOutcome {
    /// The manifest already pointed at identical bytes; nothing changed.
    Unchanged,
    /// The representation was added or replaced.
    Staged(Change),
}

fn validate_representation_key(key: &str) -> Result<(), PxError> {
    let valid = !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if valid {
        Ok(())
    } else {
        Err(PxError::Other(format!(
            "invalid representation key '{key}': use letters, digits, '_' or '-'"
        )))
    }
}

fn validate_format(format: &str) -> Result<(), PxError> {
    let valid = !format.is_empty()
        && !format.starts_with('.')
        && !format.contains("..")
        && format
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
    if valid {
        Ok(())
    } else {
        Err(PxError::Other(format!(
            "invalid representation format '{format}': use letters, digits, '.', '-' or '_'"
        )))
    }
}

/// Copy bytes into the entity directory and point `representations.<key>`
/// at them. Does not commit.
///
/// When a representation is replaced with a different format, the stale file
/// from the previous format is removed so it is not committed as an orphan.
pub fn stage_representation_bytes(
    repo: &Repository,
    manifest: &mut Manifest,
    uri: &PxUri,
    key: &str,
    bytes: &[u8],
    format: &str,
    replace: bool,
) -> Result<StageOutcome, PxError> {
    validate_representation_key(key)?;
    validate_format(format)?;
    let hash = ContentHash::from_bytes(bytes);
    let previous_file = match manifest.representations.get(key) {
        Some(existing) if existing.hash == hash.as_str() && existing.format == format => {
            debug!(key, hash = hash.as_str(), "representation unchanged");
            return Ok(StageOutcome::Unchanged);
        }
        Some(_) if !replace => {
            return Err(PxError::Other(format!(
                "representation '{key}' already exists with different content; pass --replace to overwrite"
            )));
        }
        Some(existing) => existing.uri.clone(),
        None => None,
    };

    let dir = entity_dir(repo, uri);
    std::fs::create_dir_all(&dir)?;
    let asset_filename = format!("{key}.{format}");
    std::fs::write(dir.join(&asset_filename), bytes)?;
    if let Some(previous) = previous_file.filter(|previous| {
        previous != &asset_filename && !previous.contains('/') && !previous.contains('\\')
    }) {
        let stale = dir.join(&previous);
        if stale.is_file() {
            debug!(path = %stale.display(), "removing stale representation file");
            std::fs::remove_file(stale)?;
        }
    }

    manifest.set_representation(
        key,
        Representation {
            hash: hash.as_str().to_string(),
            format: format.to_string(),
            uri: Some(asset_filename),
            tier: None,
        },
    );
    Ok(StageOutcome::Staged(Change::set(
        &format!("representations.{key}"),
        None,
        hash.as_str().to_string(),
    )))
}

/// File-based variant of [`stage_representation_bytes`].
pub fn stage_representation_file(
    repo: &Repository,
    manifest: &mut Manifest,
    uri: &PxUri,
    key: &str,
    file: &Path,
    format: &str,
    replace: bool,
) -> Result<StageOutcome, PxError> {
    let bytes = std::fs::read(file).map_err(|error| {
        PxError::Other(format!("failed to read file '{}': {error}", file.display()))
    })?;
    stage_representation_bytes(repo, manifest, uri, key, &bytes, format, replace)
}

/// Stage a representation using the existing `px add` replacement semantics.
/// An identical hash is a no-op, even when the requested format differs.
pub fn stage_add_representation(
    repo: &Repository,
    manifest: &mut Manifest,
    uri: &PxUri,
    key: &str,
    file: &Path,
    format: &str,
    replace: bool,
) -> Result<StageOutcome, PxError> {
    let hash = ContentHash::from_file(file)?;
    if let Some(existing) = manifest.representations.get(key) {
        if existing.hash == hash.as_str() {
            return Ok(StageOutcome::Unchanged);
        }
        if !replace {
            return Err(PxError::Other(format!(
                "representation '{key}' already exists with different content; pass --replace to overwrite"
            )));
        }
    }
    let dir = entity_dir(repo, uri);
    std::fs::create_dir_all(&dir)?;
    let filename = format!("{key}.{format}");
    std::fs::copy(file, dir.join(&filename))?;
    manifest.set_representation(
        key,
        Representation {
            hash: hash.as_str().to_string(),
            format: format.to_string(),
            uri: Some(filename),
            tier: None,
        },
    );
    Ok(StageOutcome::Staged(Change::set(
        &format!("representations.{key}"),
        None,
        hash.as_str().to_string(),
    )))
}

/// Infer a representation format from a file extension (`hero.PNG` → `png`).
pub fn format_from_path(path: &Path) -> Result<String, PxError> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase)
        .filter(|ext| !ext.is_empty())
        .ok_or_else(|| {
            PxError::Other(format!(
                "cannot infer a format for '{}'; add a file extension",
                path.display()
            ))
        })
}

/// One generated output to persist.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GenerationOutput {
    /// Stable semantic representation key, e.g. `portrait` or `shot_012`.
    pub key: String,
    /// Path to the generated file on the host.
    pub file: PathBuf,
    /// File format; inferred from the extension when absent.
    #[serde(default)]
    pub format: Option<String>,
}

/// Everything needed to persist a generation with full provenance.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct GenerationRecord {
    /// Generated outputs; at least one is required.
    pub outputs: Vec<GenerationOutput>,
    /// Exact prompt text sent to the model.
    #[serde(default)]
    pub prompt_text: Option<String>,
    /// Exact negative prompt, when one was used.
    #[serde(default)]
    pub negative_prompt: Option<String>,
    /// Provider returned by the generation tool.
    #[serde(default)]
    pub provider: Option<String>,
    /// Exact model identifier returned by the generation tool.
    #[serde(default)]
    pub model: Option<String>,
    /// Generation job identifier, when available.
    #[serde(default)]
    pub job_id: Option<String>,
    /// All supplied generation parameters.
    #[serde(default)]
    pub parameters: Option<Value>,
    /// Non-default model parameters.
    #[serde(default)]
    pub model_parameters: Option<Value>,
    /// Inputs: PX URI, revision, representation key, and BLAKE3 hash.
    #[serde(default)]
    pub inputs: Vec<Value>,
    /// Tool-specific provenance (for example the optimizer fragment).
    #[serde(default)]
    pub extensions: Option<Value>,
}

/// Summary of one persisted output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecordedOutput {
    pub key: String,
    pub hash: String,
    pub format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_representation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_hash: Option<String>,
}

/// Outcome of [`record_generation`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordOutcome {
    pub uri: String,
    pub branch: Option<String>,
    pub commit_id: Option<String>,
    pub manifest_version: u64,
    pub outputs: Vec<RecordedOutput>,
    /// True when every output and prompt was already recorded unchanged.
    pub unchanged: bool,
}

fn validate_record(record: &GenerationRecord) -> Result<(), PxError> {
    if record.outputs.is_empty() {
        return Err(PxError::Other("record requires at least one output".into()));
    }
    let mut seen = std::collections::BTreeSet::new();
    for output in &record.outputs {
        if !seen.insert(output.key.as_str()) {
            return Err(PxError::Other(format!(
                "duplicate output key '{}'",
                output.key
            )));
        }
        if output.key.ends_with("_prompt") {
            return Err(PxError::Other(format!(
                "output key '{}' is reserved for prompt representations",
                output.key
            )));
        }
    }
    for (name, value) in [
        ("parameters", &record.parameters),
        ("model_parameters", &record.model_parameters),
    ] {
        if value.as_ref().is_some_and(|value| !value.is_object()) {
            return Err(PxError::Other(format!("{name} must be a JSON object")));
        }
    }
    Ok(())
}

/// Persist generated outputs, prompt representations, and
/// `metadata.generation.<key>` provenance in a single commit.
///
/// This is the coded form of the PX provenance checklist:
/// - the exact prompt is saved as an addressable `<key>_prompt` representation
/// - unknown model identifiers are recorded as `unavailable`, never invented
/// - `date_created` is preserved per key and `date_updated` advances
/// - when `expected_branch` is given, the repository must be on that branch
pub fn record_generation(
    repo: &Repository,
    uri: &PxUri,
    expected_branch: Option<&str>,
    record: &GenerationRecord,
    replace: bool,
    message: Option<&str>,
    author: &str,
) -> Result<RecordOutcome, PxError> {
    validate_record(record)?;
    let branch = match expected_branch {
        Some(expected) => {
            let current = repo.current_branch()?;
            if current != expected {
                return Err(PxError::Other(format!(
                    "repository '{}' is on branch '{current}', not '{expected}'; switch first",
                    uri.repository
                )));
            }
            Some(current)
        }
        None => None,
    };

    let mut manifest = repo.read_manifest(&uri.entity_type, &uri.entity_id)?;
    // Read and validate every output before changing any repository file. A
    // missing later input must not leave earlier outputs staged on disk.
    let prepared = record.outputs.iter().map(|output| {
        validate_representation_key(&output.key)?;
        let format = match &output.format {
            Some(format) => format.to_ascii_lowercase(),
            None => format_from_path(&output.file)?,
        };
        validate_format(&format)?;
        let bytes = std::fs::read(&output.file).map_err(|error| {
            PxError::Other(format!("failed to read file '{}': {error}", output.file.display()))
        })?;
        if let Some(existing) = manifest.representations.get(&output.key)
            && (existing.hash != ContentHash::from_bytes(&bytes).as_str() || existing.format != format) && !replace {
            return Err(PxError::Other(format!("representation '{}' already exists with different content; pass --replace to overwrite", output.key)));
        }
        Ok((format, bytes))
    }).collect::<Result<Vec<_>, PxError>>()?;
    // Snapshot every file the staging step can touch so a staging error does
    // not leave an incomplete record in the checkout.
    let dir = entity_dir(repo, uri);
    let mut paths = std::collections::BTreeSet::new();
    for (output, (format, _)) in record.outputs.iter().zip(&prepared) {
        paths.insert(dir.join(format!("{}.{format}", output.key)));
        if record.prompt_text.is_some() {
            paths.insert(dir.join(format!("{}_prompt.txt", output.key)));
        }
        for key in [output.key.clone(), format!("{}_prompt", output.key)] {
            if let Some(name) = manifest
                .representations
                .get(&key)
                .and_then(|r| r.uri.as_ref())
                && !name.contains('/')
                && !name.contains('\\')
            {
                paths.insert(dir.join(name));
            }
        }
    }
    let mut snapshots = Vec::new();
    for path in paths {
        let old = match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_file() => {
                Some((std::fs::read(&path)?, metadata.permissions()))
            }
            Ok(_) => {
                return Err(PxError::Other(format!(
                    "cannot safely stage over '{}'",
                    path.display()
                )));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        snapshots.push((path, old));
    }
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let mut changes = Vec::new();
    let mut outputs = Vec::new();

    let staging: Result<(), PxError> = (|| {
        for (output, (format, bytes)) in record.outputs.iter().zip(prepared) {
            let previous_output_hash = manifest
                .representations
                .get(&output.key)
                .map(|r| r.hash.clone());
            let outcome = stage_representation_bytes(
                repo,
                &mut manifest,
                uri,
                &output.key,
                &bytes,
                &format,
                replace,
            )?;
            if let StageOutcome::Staged(change) = outcome {
                changes.push(change);
            }
            let hash = manifest.representations[&output.key].hash.clone();
            if previous_output_hash
                .as_ref()
                .is_some_and(|old| old != &hash)
                && manifest
                    .metadata
                    .get("feedback")
                    .and_then(|v| v.get(output.key.as_str()))
                    .is_some()
            {
                let stale_key = format!("metadata.feedback.{}.stale", output.key);
                set_manifest_value(&mut manifest, &stale_key, serde_yaml::Value::Bool(true))?;
                changes.push(Change::set(&stale_key, None, "true".to_string()));
            }

            let (prompt_representation, prompt_hash) = match &record.prompt_text {
                Some(prompt) => {
                    let prompt_key = format!("{}_prompt", output.key);
                    if let StageOutcome::Staged(change) = stage_representation_bytes(
                        repo,
                        &mut manifest,
                        uri,
                        &prompt_key,
                        prompt.as_bytes(),
                        "txt",
                        true,
                    )? {
                        changes.push(change);
                    }
                    (
                        Some(prompt_key),
                        Some(ContentHash::from_str_content(prompt).as_str().to_string()),
                    )
                }
                None => (None, None),
            };

            let metadata_key = format!("metadata.generation.{}", output.key);
            let existing = manifest
                .metadata
                .get("generation")
                .and_then(|generation| generation.get(output.key.as_str()))
                .cloned();
            let date_created = existing
                .as_ref()
                .and_then(|value| value.get("date_created"))
                .and_then(serde_yaml::Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| now.clone());
            let entry = json!({
                "provider": record.provider.clone().unwrap_or_else(|| UNAVAILABLE.to_string()),
                "model": record.model.clone().unwrap_or_else(|| UNAVAILABLE.to_string()),
                "output_hash": hash,
                "format": format,
                "prompt_text": record.prompt_text,
                "prompt_hash": prompt_hash,
                "prompt_representation": prompt_representation,
                "negative_prompt": record.negative_prompt,
                "parameters": record.parameters.clone().unwrap_or_else(|| json!({})),
                "model_parameters": record.model_parameters.clone().unwrap_or_else(|| json!({})),
                "inputs": record.inputs,
                "job_id": record.job_id,
                "extensions": record.extensions,
                "date_created": date_created,
                "date_updated": now,
            });
            let entry = strip_nulls(entry);
            set_manifest_value(
                &mut manifest,
                &metadata_key,
                serde_yaml::to_value(&entry)
                    .map_err(|e| PxError::ManifestValidationError(e.to_string()))?,
            )?;
            changes.push(Change::set(&metadata_key, None, entry.to_string()));

            outputs.push(RecordedOutput {
                key: output.key.clone(),
                hash,
                format,
                prompt_representation,
                prompt_hash,
            });
        }

        crate::schema::validate_manifest(&manifest)
            .map_err(|errors| PxError::ManifestValidationError(errors.join("; ")))?;
        Ok(())
    })();
    if let Err(error) = staging {
        for (path, old) in snapshots {
            let restored = match old {
                Some((bytes, permissions)) => std::fs::write(&path, bytes)
                    .and_then(|()| std::fs::set_permissions(&path, permissions)),
                None => match std::fs::remove_file(&path) {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                    result => result,
                },
            };
            if let Err(restore_error) = restored {
                return Err(PxError::Other(format!(
                    "{error}; failed to restore {}: {restore_error}",
                    path.display()
                )));
            }
        }
        return Err(error);
    }

    let unchanged = !changes
        .iter()
        .any(|change| change.path.starts_with("representations."));
    let keys: Vec<&str> = outputs.iter().map(|output| output.key.as_str()).collect();
    let message = message
        .map(str::to_string)
        .unwrap_or_else(|| format!("record generation {} on {}", keys.join(", "), uri.entity_id));
    let commit: Commit = repo.commit_manifest(&mut manifest, &message, author, changes)?;
    info!(
        uri = %uri.identity(),
        commit = %commit.id,
        outputs = keys.len(),
        "recorded generation"
    );

    Ok(RecordOutcome {
        uri: uri.identity(),
        branch,
        commit_id: Some(commit.id),
        manifest_version: manifest.version,
        outputs,
        unchanged,
    })
}

fn strip_nulls(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .filter(|(_, value)| !value.is_null())
                .map(|(key, value)| (key, strip_nulls(value)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(strip_nulls).collect()),
        other => other,
    }
}

/// Feedback captured for training export only when explicitly opted in.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedbackRecord {
    pub uri: String,
    pub branch: String,
    pub key: String,
    pub accepted: bool,
    pub rating: u8,
    #[serde(default)]
    pub per_criterion: std::collections::BTreeMap<String, u8>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub export_training: bool,
}
impl FeedbackRecord {
    pub fn validate(&self) -> Result<PxUri, PxError> {
        validate_representation_key(&self.key)?;
        if self.branch.trim().is_empty()
            || !(1..=5).contains(&self.rating)
            || self
                .per_criterion
                .iter()
                .any(|(key, value)| key.trim().is_empty() || !(1..=5).contains(value))
        {
            return Err(PxError::Other(
                "feedback requires a branch and integer ratings from 1 to 5".into(),
            ));
        }
        let uri: PxUri = self.uri.parse()?;
        if uri.fragment.is_some() {
            return Err(PxError::Other(
                "feedback URI must identify an entity".into(),
            ));
        }
        Ok(uri)
    }
}

/// Validate the active branch and target before writing feedback in one commit.
pub fn record_feedback_on_branch(
    repo: &Repository,
    record: &FeedbackRecord,
    author: &str,
) -> Result<String, PxError> {
    let uri = record.validate()?;
    if repo.current_branch()? != record.branch {
        return Err(PxError::Other(
            "switch to the requested feedback branch first".into(),
        ));
    }
    let manifest = repo.read_manifest(&uri.entity_type, &uri.entity_id)?;
    if !manifest.representations.contains_key(&record.key) {
        return Err(PxError::Other(
            "feedback representation does not exist".into(),
        ));
    }
    let mut feedback = serde_json::to_value(record).map_err(|e| PxError::Other(e.to_string()))?;
    let fields = feedback
        .as_object_mut()
        .expect("feedback serializes to an object");
    fields.remove("uri");
    fields.remove("branch");
    fields.remove("key");
    fields.insert(
        "output_hash".into(),
        Value::String(manifest.representations[&record.key].hash.clone()),
    );
    fields.insert("stale".into(), Value::Bool(false));
    fields.insert(
        "date_updated".into(),
        Value::String(chrono::Utc::now().to_rfc3339()),
    );
    record_feedback(repo, &uri, &record.key, &feedback, author)
}

/// Write feedback for one representation key under `metadata.feedback.<key>`
/// and commit. Returns the commit ID.
pub fn record_feedback(
    repo: &Repository,
    uri: &PxUri,
    key: &str,
    feedback: &Value,
    author: &str,
) -> Result<String, PxError> {
    validate_representation_key(key)?;
    if !feedback.is_object() {
        return Err(PxError::Other("feedback must be a JSON object".into()));
    }
    let mut manifest = repo.read_manifest(&uri.entity_type, &uri.entity_id)?;
    let metadata_key = format!("metadata.feedback.{key}");
    set_manifest_value(
        &mut manifest,
        &metadata_key,
        serde_yaml::to_value(feedback)
            .map_err(|e| PxError::ManifestValidationError(e.to_string()))?,
    )?;
    let commit = repo.commit_manifest(
        &mut manifest,
        &format!("record feedback for {key} on {}", uri.entity_id),
        author,
        vec![Change::set(&metadata_key, None, feedback.to_string())],
    )?;
    Ok(commit.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::mock_repo;
    use crate::types::EntityType;
    use tempfile::TempDir;

    fn setup() -> (TempDir, Repository, PxUri) {
        let tmp = TempDir::new().unwrap();
        let repo = mock_repo(&tmp);
        repo.create_entity(&EntityType::from("character"), "atlas", "Atlas", "test")
            .unwrap();
        let uri: PxUri = "px://testverse/character/atlas".parse().unwrap();
        (tmp, repo, uri)
    }

    fn write(tmp: &TempDir, name: &str, bytes: &[u8]) -> PathBuf {
        let path = tmp.path().join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn normalizes_bare_and_sectioned_keys() {
        assert_eq!(normalized_manifest_path("height"), "properties.height");
        assert_eq!(normalized_manifest_path("metadata.a.b"), "metadata.a.b");
        assert_eq!(
            normalized_manifest_path("propertiesX"),
            "properties.propertiesX"
        );
    }

    #[test]
    fn parses_json_values_or_falls_back_to_strings() {
        assert_eq!(
            yaml_from_json_or_string("{\"a\":1}")["a"],
            serde_yaml::Value::from(1)
        );
        assert_eq!(
            yaml_from_json_or_string("173 cm"),
            serde_yaml::Value::String("173 cm".into())
        );
    }

    #[test]
    fn set_and_unset_round_trip() {
        let mut manifest = Manifest::new("r", EntityType::from("character"), "a", "A");
        set_manifest_value(&mut manifest, "metadata.x.y", "1".into()).unwrap();
        assert!(manifest.metadata["x"]["y"].is_string());
        set_manifest_value(&mut manifest, "height", "2".into()).unwrap();
        assert!(manifest.properties.contains_key("height"));
        unset_manifest_value(&mut manifest, "metadata.x.y").unwrap();
        assert!(unset_manifest_value(&mut manifest, "metadata.x.y").is_err());
        assert!(unset_manifest_value(&mut manifest, "metadata.missing.y").is_err());
        assert!(set_manifest_value(&mut manifest, "metadata..y", "1".into()).is_err());
    }

    #[test]
    fn set_rejects_traversal_through_scalars() {
        let mut manifest = Manifest::new("r", EntityType::from("character"), "a", "A");
        set_manifest_value(&mut manifest, "metadata.x", "scalar".into()).unwrap();
        assert!(set_manifest_value(&mut manifest, "metadata.x.y", "1".into()).is_err());
    }

    #[test]
    fn infers_formats() {
        assert_eq!(format_from_path(Path::new("a/b.PNG")).unwrap(), "png");
        assert!(format_from_path(Path::new("noext")).is_err());
    }

    #[test]
    fn stage_is_idempotent_and_guards_replacement() {
        let (tmp, repo, uri) = setup();
        let mut manifest = repo
            .read_manifest(&uri.entity_type, &uri.entity_id)
            .unwrap();
        let first =
            stage_representation_bytes(&repo, &mut manifest, &uri, "portrait", b"a", "png", false)
                .unwrap();
        assert!(matches!(first, StageOutcome::Staged(_)));
        let again =
            stage_representation_bytes(&repo, &mut manifest, &uri, "portrait", b"a", "png", false)
                .unwrap();
        assert!(matches!(again, StageOutcome::Unchanged));
        assert!(
            stage_representation_bytes(&repo, &mut manifest, &uri, "portrait", b"b", "png", false)
                .is_err()
        );
        // Replacing with a new format removes the stale file.
        stage_representation_bytes(&repo, &mut manifest, &uri, "portrait", b"b", "jpg", true)
            .unwrap();
        let dir = entity_dir(&repo, &uri);
        assert!(dir.join("portrait.jpg").is_file());
        assert!(!dir.join("portrait.png").exists());
        drop(tmp);
    }

    #[test]
    fn stage_validates_key_and_format() {
        let (_tmp, repo, uri) = setup();
        let mut manifest = repo
            .read_manifest(&uri.entity_type, &uri.entity_id)
            .unwrap();
        assert!(
            stage_representation_bytes(&repo, &mut manifest, &uri, "../x", b"a", "png", false)
                .is_err()
        );
        assert!(
            stage_representation_bytes(&repo, &mut manifest, &uri, "ok", b"a", "p/ng", false)
                .is_err()
        );
        assert!(
            stage_representation_file(
                &repo,
                &mut manifest,
                &uri,
                "ok",
                Path::new("/nonexistent/x.png"),
                "png",
                false
            )
            .is_err()
        );
    }

    #[test]
    fn add_preserves_existing_replacement_behavior() {
        let (tmp, repo, uri) = setup();
        let mut manifest = repo
            .read_manifest(&uri.entity_type, &uri.entity_id)
            .unwrap();
        let first = write(&tmp, "first.png", b"first");
        let second = write(&tmp, "second.jpg", b"second");
        assert!(matches!(
            stage_add_representation(&repo, &mut manifest, &uri, "portrait", &first, "png", false)
                .unwrap(),
            StageOutcome::Staged(_)
        ));
        assert!(matches!(
            stage_add_representation(&repo, &mut manifest, &uri, "portrait", &first, "jpg", false)
                .unwrap(),
            StageOutcome::Unchanged
        ));
        assert!(
            stage_add_representation(
                &repo,
                &mut manifest,
                &uri,
                "portrait",
                &second,
                "jpg",
                false
            )
            .is_err()
        );
        stage_add_representation(&repo, &mut manifest, &uri, "portrait", &second, "jpg", true)
            .unwrap();
        assert!(entity_dir(&repo, &uri).join("portrait.png").exists());
        assert!(entity_dir(&repo, &uri).join("portrait.jpg").exists());
    }

    #[test]
    fn records_outputs_prompts_and_provenance_in_one_commit() {
        let (tmp, repo, uri) = setup();
        let image = write(&tmp, "out.PNG", b"image-bytes");
        let record = GenerationRecord {
            outputs: vec![GenerationOutput {
                key: "portrait".into(),
                file: image.clone(),
                format: None,
            }],
            prompt_text: Some("A tall explorer".into()),
            negative_prompt: Some("blurry".into()),
            provider: Some("google".into()),
            model: None,
            job_id: Some("job-1".into()),
            parameters: Some(json!({"aspect_ratio": "1:1"})),
            model_parameters: None,
            inputs: vec![
                json!({"uri": "px://testverse/character/atlas", "key": "character_sheet"}),
            ],
            extensions: Some(json!({"px_optimizer": {"artifact": "prompt_author@v0"}})),
        };
        let outcome = record_generation(&repo, &uri, None, &record, false, None, "test").unwrap();
        assert_eq!(outcome.outputs.len(), 1);
        assert!(!outcome.unchanged);
        let manifest = repo
            .read_manifest(&uri.entity_type, &uri.entity_id)
            .unwrap();
        assert_eq!(manifest.representations["portrait"].format, "png");
        assert_eq!(manifest.representations["portrait_prompt"].format, "txt");
        let generation = &manifest.metadata["generation"]["portrait"];
        assert_eq!(generation["model"].as_str(), Some(UNAVAILABLE));
        assert_eq!(generation["provider"].as_str(), Some("google"));
        assert!(generation.get("model_parameters").is_some());
        assert!(generation["extensions"]["px_optimizer"].is_mapping());
        let created = generation["date_created"].as_str().unwrap().to_string();

        // Re-recording identical bytes preserves date_created and reports unchanged.
        let again =
            record_generation(&repo, &uri, None, &record, false, Some("again"), "test").unwrap();
        assert!(again.unchanged);
        let manifest = repo
            .read_manifest(&uri.entity_type, &uri.entity_id)
            .unwrap();
        assert_eq!(
            manifest.metadata["generation"]["portrait"]["date_created"].as_str(),
            Some(created.as_str())
        );
    }

    #[test]
    fn record_validates_input() {
        let (tmp, repo, uri) = setup();
        let file = write(&tmp, "a.png", b"a");
        let empty = GenerationRecord::default();
        assert!(record_generation(&repo, &uri, None, &empty, false, None, "t").is_err());
        let output = GenerationOutput {
            key: "a".into(),
            file: file.clone(),
            format: None,
        };
        let duplicate = GenerationRecord {
            outputs: vec![output.clone(), output.clone()],
            ..Default::default()
        };
        assert!(record_generation(&repo, &uri, None, &duplicate, false, None, "t").is_err());
        let reserved = GenerationRecord {
            outputs: vec![GenerationOutput {
                key: "a_prompt".into(),
                file: file.clone(),
                format: None,
            }],
            ..Default::default()
        };
        assert!(record_generation(&repo, &uri, None, &reserved, false, None, "t").is_err());
        let bad_params = GenerationRecord {
            outputs: vec![output],
            parameters: Some(json!([1])),
            ..Default::default()
        };
        assert!(record_generation(&repo, &uri, None, &bad_params, false, None, "t").is_err());
    }

    #[test]
    fn record_checks_all_outputs_before_staging() {
        let (tmp, repo, uri) = setup();
        let first = write(&tmp, "first.png", b"first");
        let record = GenerationRecord {
            outputs: vec![
                GenerationOutput {
                    key: "first".into(),
                    file: first,
                    format: None,
                },
                GenerationOutput {
                    key: "second".into(),
                    file: tmp.path().join("missing.png"),
                    format: None,
                },
            ],
            prompt_text: Some("prompt".into()),
            ..Default::default()
        };
        assert!(record_generation(&repo, &uri, None, &record, false, None, "test").is_err());
        assert!(!entity_dir(&repo, &uri).join("first.png").exists());
        assert!(!entity_dir(&repo, &uri).join("first_prompt.txt").exists());
    }

    #[test]
    fn record_restores_staged_files_on_metadata_error() {
        let (tmp, repo, uri) = setup();
        let mut manifest = repo
            .read_manifest(&uri.entity_type, &uri.entity_id)
            .unwrap();
        set_manifest_value(&mut manifest, "metadata.generation", "not an object".into()).unwrap();
        repo.commit_manifest(&mut manifest, "invalid metadata", "test", vec![])
            .unwrap();
        let dir = entity_dir(&repo, &uri);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("portrait.png"), b"existing working file").unwrap();
        let record = GenerationRecord {
            outputs: vec![GenerationOutput {
                key: "portrait".into(),
                file: write(&tmp, "new.png", b"new"),
                format: None,
            }],
            prompt_text: Some("prompt".into()),
            ..Default::default()
        };
        assert!(record_generation(&repo, &uri, None, &record, false, None, "test").is_err());
        assert_eq!(
            std::fs::read(dir.join("portrait.png")).unwrap(),
            b"existing working file"
        );
        assert!(!dir.join("portrait_prompt.txt").exists());
    }

    #[test]
    fn record_enforces_expected_branch() {
        let (tmp, repo, uri) = setup();
        let file = write(&tmp, "a.png", b"a");
        let record = GenerationRecord {
            outputs: vec![GenerationOutput {
                key: "a".into(),
                file,
                format: None,
            }],
            ..Default::default()
        };
        let current = repo.current_branch().unwrap();
        assert!(
            record_generation(
                &repo,
                &uri,
                Some("definitely-not-a-branch"),
                &record,
                false,
                None,
                "t"
            )
            .is_err()
        );
        let outcome =
            record_generation(&repo, &uri, Some(&current), &record, false, None, "t").unwrap();
        assert_eq!(outcome.branch.as_deref(), Some(current.as_str()));
    }

    #[test]
    fn feedback_is_written_under_metadata() {
        let (_tmp, repo, uri) = setup();
        record_feedback(
            &repo,
            &uri,
            "portrait",
            &json!({"rating": 5, "accepted": true}),
            "t",
        )
        .unwrap();
        let manifest = repo
            .read_manifest(&uri.entity_type, &uri.entity_id)
            .unwrap();
        assert_eq!(
            manifest.metadata["feedback"]["portrait"]["rating"].as_u64(),
            Some(5)
        );
        assert!(record_feedback(&repo, &uri, "portrait", &json!(1), "t").is_err());
        assert!(record_feedback(&repo, &uri, "bad key", &json!({}), "t").is_err());
    }

    #[test]
    fn branch_feedback_validates_ratings_target_and_export_opt_in() {
        let (_tmp, repo, uri) = setup();
        let branch = repo.current_branch().unwrap();
        let mut record = FeedbackRecord {
            uri: uri.to_string(),
            branch,
            key: "portrait".into(),
            accepted: true,
            rating: 5,
            per_criterion: Default::default(),
            notes: None,
            export_training: false,
        };
        assert!(record_feedback_on_branch(&repo, &record, "t").is_err());
        let mut manifest = repo
            .read_manifest(&uri.entity_type, &uri.entity_id)
            .unwrap();
        stage_representation_bytes(
            &repo,
            &mut manifest,
            &uri,
            "portrait",
            b"image",
            "png",
            false,
        )
        .unwrap();
        manifest
            .to_file(&repo.manifest_path(&uri.entity_type, &uri.entity_id))
            .unwrap();
        record.rating = 6;
        assert!(record_feedback_on_branch(&repo, &record, "t").is_err());
        record.rating = 5;
        record_feedback_on_branch(&repo, &record, "t").unwrap();
        let manifest = repo
            .read_manifest(&uri.entity_type, &uri.entity_id)
            .unwrap();
        assert_eq!(
            manifest.metadata["feedback"]["portrait"]["export_training"].as_bool(),
            Some(false)
        );
        record.branch = "wrong".into();
        assert!(record_feedback_on_branch(&repo, &record, "t").is_err());
    }

    #[test]
    fn pinned_input_verification_checks_manifest_and_bytes() {
        let (_tmp, repo, uri) = setup();
        let mut manifest = repo
            .read_manifest(&uri.entity_type, &uri.entity_id)
            .unwrap();
        stage_representation_bytes(
            &repo,
            &mut manifest,
            &uri,
            "reference",
            b"reference",
            "txt",
            false,
        )
        .unwrap();
        repo.commit_manifest(&mut manifest, "reference", "test", vec![])
            .unwrap();
        let hash = ContentHash::from_bytes(b"reference");
        repo.verify_representation_at_ref(&uri, "reference", "revision", hash.as_str())
            .unwrap();
        assert!(
            repo.verify_representation_at_ref(
                &uri,
                "reference",
                "revision",
                ContentHash::from_bytes(b"wrong").as_str()
            )
            .is_err()
        );
        std::fs::write(entity_dir(&repo, &uri).join("reference.txt"), b"tampered").unwrap();
        assert!(
            repo.verify_representation_at_ref(&uri, "reference", "revision", hash.as_str())
                .is_err()
        );
    }
}
