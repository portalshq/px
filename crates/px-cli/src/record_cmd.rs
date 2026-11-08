use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use px_core::{
    ContentHash,
    mutation::{GenerationOutput, GenerationRecord, record_generation},
};
use serde_json::Value;

pub struct Arguments {
    pub uri: String,
    pub branch: String,
    pub output: Vec<String>,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub parameters: String,
    pub model_parameters: String,
    pub input_refs: String,
    pub job_id: Option<String>,
    pub provenance_json: Option<String>,
    pub replace: bool,
    pub message: Option<String>,
    pub author: String,
}

fn object_json(value: &str, name: &str) -> Result<Value> {
    let value: Value =
        serde_json::from_str(value).with_context(|| format!("invalid {name} JSON"))?;
    if !value.is_object() {
        bail!("{name} must be a JSON object");
    }
    Ok(value)
}

fn parse_inputs(value: &str) -> Result<Vec<Value>> {
    let inputs: Vec<Value> =
        serde_json::from_str(value).context("input references must be a JSON array")?;
    for input in &inputs {
        for key in ["uri", "key", "hash", "commit"] {
            if input
                .get(key)
                .and_then(Value::as_str)
                .is_none_or(str::is_empty)
            {
                bail!("each input reference requires a nonempty {key}");
            }
        }
        super::parse_entity_uri(input["uri"].as_str().unwrap())?;
        ContentHash::parse(input["hash"].as_str().unwrap())?;
        let commit = input["commit"].as_str().unwrap();
        if commit.len() < 32 || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            bail!("input commit must be an immutable revision signature");
        }
    }
    Ok(inputs)
}

pub fn run(base_dir: &Path, args: Arguments) -> Result<()> {
    let uri = super::parse_entity_uri(&args.uri)?;
    let repo = super::open_repo(base_dir, &uri.repository)?;
    let outputs = args
        .output
        .iter()
        .map(|output| {
            let (key, file) = output.split_once('=').context("output must be KEY=PATH")?;
            if key.is_empty() || file.is_empty() {
                bail!("output must be KEY=PATH");
            }
            Ok(GenerationOutput {
                key: key.into(),
                file: PathBuf::from(file),
                format: None,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let record = GenerationRecord {
        outputs,
        prompt_text: Some(args.prompt),
        negative_prompt: args.negative_prompt,
        provider: args.provider,
        model: args.model,
        parameters: Some(object_json(&args.parameters, "parameters")?),
        model_parameters: Some(object_json(&args.model_parameters, "model parameters")?),
        inputs: parse_inputs(&args.input_refs)?,
        job_id: args.job_id,
        extensions: args
            .provenance_json
            .as_deref()
            .map(|s| object_json(s, "provenance"))
            .transpose()?,
    };
    for input in &record.inputs {
        let input_uri = super::parse_entity_uri(input["uri"].as_str().unwrap())?;
        let input_repo = super::open_repo(base_dir, &input_uri.repository)?;
        input_repo
            .verify_representation_at_ref(
                &input_uri,
                input["key"].as_str().unwrap(),
                input["commit"].as_str().unwrap(),
                input["hash"].as_str().unwrap(),
            )
            .context("failed to verify pinned input")?;
    }
    let result = record_generation(
        &repo,
        &uri,
        Some(&args.branch),
        &record,
        args.replace,
        args.message.as_deref(),
        &args.author,
    )?;
    let manifest = repo.read_manifest(&uri.entity_type, &uri.entity_id)?;
    let dir = px_core::mutation::entity_dir(&repo, &uri);
    for output in &result.outputs {
        for (key, hash) in std::iter::once((&output.key, &output.hash)).chain(
            output
                .prompt_representation
                .iter()
                .zip(output.prompt_hash.iter()),
        ) {
            let repr = manifest
                .representations
                .get(key)
                .with_context(|| format!("committed representation {key} is missing"))?;
            if repr.hash != *hash {
                bail!("committed hash mismatch for {key}");
            }
            let filename = repr
                .uri
                .as_deref()
                .context("committed representation has no file")?;
            let actual = ContentHash::from_file(&dir.join(filename))?;
            if actual.as_str() != hash {
                bail!("file hash mismatch for {key}");
            }
        }
    }
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn input_references_need_pins_and_valid_hashes() {
        assert!(parse_inputs("[]").unwrap().is_empty());
        assert!(parse_inputs(r#"[{"uri":"px://repo/character/a"}]"#).is_err());
        assert!(object_json("[]", "parameters").is_err());
    }
}
