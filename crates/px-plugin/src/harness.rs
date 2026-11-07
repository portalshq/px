//! Harness-neutral skill installation. Hook settings are never edited.
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub trait HarnessAdapter {
    fn install_skills(&self, plugin_id: &str, skills: &[PathBuf]) -> io::Result<()>;
    fn uninstall_skills(&self, plugin_id: &str, names: &[String]) -> io::Result<()>;
    fn hook_instructions(&self) -> String;
}

pub struct DirectoryAdapter {
    pub name: String,
    pub skills_dir: PathBuf,
}
fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' || c == b'_')
}
fn copy_dir(source: &Path, target: &Path) -> io::Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let destination = target.join(entry.file_name());
        if kind.is_dir() {
            fs::create_dir(&destination)?;
            copy_dir(&entry.path(), &destination)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), &destination)?;
        } else {
            return Err(io::Error::other("skill contains a non-regular file"));
        }
    }
    Ok(())
}
impl HarnessAdapter for DirectoryAdapter {
    fn install_skills(&self, plugin_id: &str, skills: &[PathBuf]) -> io::Result<()> {
        fs::create_dir_all(&self.skills_dir)?;
        for source in skills {
            if !fs::symlink_metadata(source)?.file_type().is_dir() {
                return Err(io::Error::other("skill source must be a directory"));
            }
            let name = source
                .file_name()
                .and_then(|n| n.to_str())
                .filter(|s| safe_name(s))
                .ok_or_else(|| io::Error::other("invalid skill name"))?;
            if !source.join("SKILL.md").is_file() {
                return Err(io::Error::other("skill must contain SKILL.md"));
            }
            let destination = self.skills_dir.join(name);
            let owned = match fs::symlink_metadata(&destination) {
                Ok(metadata) if metadata.file_type().is_dir() => {
                    if fs::read_to_string(destination.join(".px-owner"))?.as_str() != plugin_id {
                        return Err(io::Error::other("skill is not owned by this plugin"));
                    }
                    true
                }
                Ok(_) => return Err(io::Error::other("skill destination is not a directory")),
                Err(error) if error.kind() == io::ErrorKind::NotFound => false,
                Err(error) => return Err(error),
            };
            let staging = tempfile::tempdir_in(&self.skills_dir)?;
            let payload = staging.path().join("payload");
            fs::create_dir(&payload)?;
            copy_dir(source, &payload)?;
            {
                use std::io::Write;
                let mut marker = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(payload.join(".px-owner"))?;
                marker.write_all(plugin_id.as_bytes())?;
            }
            let backup = staging.path().join("previous");
            if owned {
                fs::rename(&destination, &backup)?;
            }
            if let Err(error) = fs::rename(&payload, &destination) {
                if owned {
                    fs::rename(&backup, &destination)?;
                }
                return Err(error);
            }
        }
        Ok(())
    }
    fn uninstall_skills(&self, plugin_id: &str, names: &[String]) -> io::Result<()> {
        for name in names {
            if !safe_name(name) {
                return Err(io::Error::other("invalid skill name"));
            }
            let dir = self.skills_dir.join(name);
            match fs::symlink_metadata(&dir) {
                Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e),
                Ok(metadata) if !metadata.file_type().is_dir() => {
                    return Err(io::Error::other("skill path is not a directory"));
                }
                Ok(_) => {}
            }
            if fs::read_to_string(dir.join(".px-owner"))? != plugin_id {
                return Err(io::Error::other("skill is not owned by this plugin"));
            }
            fs::remove_dir_all(dir)?;
        }
        Ok(())
    }
    fn hook_instructions(&self) -> String {
        format!(
            "Configure {} hooks manually; PX does not edit harness settings.",
            self.name
        )
    }
}

/// Detect harness directories; the shared agents directory is always included.
pub fn detect(home: &Path) -> Vec<DirectoryAdapter> {
    [
        ("agents", ".agents/skills", true),
        ("claude-code", ".claude/skills", false),
        ("cursor", ".cursor/skills", false),
        ("devin", ".config/devin/skills", false),
    ]
    .into_iter()
    .filter(|(_, path, always)| *always || home.join(path).parent().is_some_and(Path::is_dir))
    .map(|(name, path, _)| DirectoryAdapter {
        name: name.into(),
        skills_dir: home.join(path),
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_owned_skills_are_removed() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source/my-skill");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("SKILL.md"), "instructions").unwrap();
        let adapter = DirectoryAdapter {
            name: "test".into(),
            skills_dir: root.path().join("skills"),
        };
        adapter
            .install_skills("px.example", std::slice::from_ref(&source))
            .unwrap();
        adapter
            .install_skills("px.example", std::slice::from_ref(&source))
            .unwrap();
        fs::write(source.join("SKILL.md"), "updated instructions").unwrap();
        adapter
            .install_skills("px.example", std::slice::from_ref(&source))
            .unwrap();
        assert_eq!(
            fs::read_to_string(adapter.skills_dir.join("my-skill/SKILL.md")).unwrap(),
            "updated instructions"
        );
        assert!(
            adapter
                .install_skills("other", std::slice::from_ref(&source))
                .is_err()
        );
        assert!(
            adapter
                .uninstall_skills("other", &["my-skill".into()])
                .is_err()
        );
        adapter
            .uninstall_skills("px.example", &["my-skill".into()])
            .unwrap();
        assert!(!adapter.skills_dir.join("my-skill").exists());
    }
}
