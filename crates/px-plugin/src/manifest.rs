use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PluginManifest {
    pub id: String,
    pub namespace: String,
    pub version: String,
    pub px: String,
    pub license: String,
    pub kind: PluginKind,
    pub entrypoint: BTreeMap<String, String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    pub tools: Vec<PluginTool>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub hooks: Vec<Hook>,
    pub commerce: Commerce,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PluginKind {
    Executable,
    UvPython,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PluginTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    #[serde(default = "empty_annotations")]
    pub annotations: Value,
    pub timeout_ms: u64,
}

fn empty_annotations() -> Value {
    Value::Object(Default::default())
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Hook {
    pub event: HookEvent,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    SessionStart,
    PreGenerate,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "model", rename_all = "snake_case", deny_unknown_fields)]
pub enum Commerce {
    Free,
    OneTime { product: String, major: u64 },
    Usage { meters: Vec<String> },
}

#[derive(Debug, thiserror::Error)]
#[error("invalid plugin manifest: {0}")]
pub struct ManifestError(pub String);

fn safe_component(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && s.as_bytes()[0].is_ascii_lowercase()
}
fn safe_tool(s: &str) -> bool {
    !s.is_empty()
        && s.as_bytes()[0].is_ascii_lowercase()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}
fn safe_relative(s: &str) -> bool {
    use std::path::{Component, Path};
    !s.is_empty()
        && !s.contains('\\')
        && Path::new(s)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}
/// Validate a reverse-DNS plugin identity before using it in a filesystem path.
pub fn valid_plugin_id(id: &str) -> bool {
    id.split('.').count() >= 2 && id.split('.').all(safe_component)
}

impl PluginManifest {
    pub fn validate(&self, occupied_namespaces: &BTreeSet<String>) -> Result<(), ManifestError> {
        let invalid = |message: &str| ManifestError(message.into());
        if !valid_plugin_id(&self.id) {
            return Err(invalid("id must be a reverse-DNS identifier"));
        }
        if !safe_component(&self.namespace) || self.namespace.contains('-') {
            return Err(invalid("invalid namespace"));
        }
        const RESERVED: &[&str] = &[
            "add",
            "auth",
            "backend",
            "branch",
            "choose",
            "commit",
            "configure",
            "content",
            "create",
            "delete",
            "doctor",
            "head",
            "history",
            "install",
            "license",
            "list",
            "log",
            "merge",
            "plugin",
            "presign",
            "pull",
            "push",
            "query",
            "record",
            "remote",
            "resolve",
            "revert",
            "schema",
            "server",
            "set",
            "sign",
            "status",
            "switch",
            "sync",
            "tag",
            "unset",
            "validate",
            "verify",
        ];
        if RESERVED.contains(&self.namespace.as_str())
            || occupied_namespaces.contains(&self.namespace)
        {
            return Err(invalid("namespace is reserved or already in use"));
        }
        semver::Version::parse(&self.version).map_err(|_| invalid("invalid version"))?;
        semver::VersionReq::parse(&self.px)
            .map_err(|_| invalid("invalid PX version requirement"))?;
        if self.license.trim().is_empty() {
            return Err(invalid("license is required"));
        }
        if self.entrypoint.is_empty() || self.entrypoint.values().any(|s| !safe_relative(s)) {
            return Err(invalid("entrypoints must be relative paths"));
        }
        if self.skills.iter().any(|s| !safe_relative(s)) {
            return Err(invalid("skills must be relative paths"));
        }
        if self
            .capabilities
            .iter()
            .any(|s| !matches!(s.as_str(), "px-read" | "llm" | "network"))
        {
            return Err(invalid("unknown or reserved capability"));
        }
        let mut names = BTreeSet::new();
        for tool in &self.tools {
            if !safe_tool(&tool.name)
                || format!("px_{}_{}", self.namespace, tool.name).len() > 64
                || !names.insert(tool.name.replace('-', "_"))
            {
                return Err(invalid("invalid or duplicate tool name"));
            }
            if tool.description.trim().is_empty()
                || tool.timeout_ms == 0
                || !tool.annotations.is_object()
                || !tool.input_schema.is_object()
                || tool.input_schema.get("type").and_then(Value::as_str) != Some("object")
            {
                return Err(invalid(
                    "tool requires a description, timeout and object input schema",
                ));
            }
        }
        match &self.commerce {
            Commerce::OneTime { product, .. } if product.is_empty() => {
                return Err(invalid("product is required"));
            }
            Commerce::Usage { meters }
                if meters.is_empty() || meters.iter().any(|s| s.is_empty()) =>
            {
                return Err(invalid("usage meters are required"));
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> PluginManifest {
        serde_yaml::from_str(
            r#"
id: px.optimizer
namespace: optimize
version: 0.1.0
px: ">=0.9, <0.11"
license: PolyForm-Shield-1.0.0
kind: executable
entrypoint: {aarch64-apple-darwin: bin/optimizer}
capabilities: [px-read, llm]
tools:
  - name: context
    description: Assemble context
    input_schema: {type: object}
    timeout_ms: 5000
skills: [skills/px-optimize]
commerce: {model: free}
"#,
        )
        .unwrap()
    }
    #[test]
    fn manifest_validation_rejects_unsafe_or_conflicting_entries() {
        let mut m = sample();
        assert!(m.validate(&BTreeSet::new()).is_ok());
        assert!(m.validate(&BTreeSet::from(["optimize".into()])).is_err());
        m.namespace = "record".into();
        assert!(m.validate(&BTreeSet::new()).is_err());
        m = sample();
        m.entrypoint.insert("test".into(), "../escape".into());
        assert!(m.validate(&BTreeSet::new()).is_err());
        m = sample();
        m.capabilities.push("px-write".into());
        assert!(m.validate(&BTreeSet::new()).is_err());
        m = sample();
        m.tools.push(m.tools[0].clone());
        assert!(m.validate(&BTreeSet::new()).is_err());
        m = sample();
        m.tools[0].name = "a".repeat(65);
        assert!(m.validate(&BTreeSet::new()).is_err());
    }
}
