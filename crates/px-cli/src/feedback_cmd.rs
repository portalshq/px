use anyhow::{Context, Result};
use px_core::mutation::{FeedbackRecord, record_feedback_on_branch};
use std::path::Path;

pub fn run(base_dir: &Path, input: &str) -> Result<()> {
    let feedback: FeedbackRecord = serde_json::from_str(input).context("invalid feedback input")?;
    let uri = feedback.validate()?;
    let repo = super::open_repo(base_dir, &uri.repository)?;
    let commit = record_feedback_on_branch(&repo, &feedback, "px.optimizer")?;
    println!(
        "{}",
        serde_json::json!({"uri":uri.to_string(),"branch":feedback.branch,"key":feedback.key,"commit":commit})
    );
    Ok(())
}
