use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Manifest {
    pub version: u32,
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub plugins: BTreeMap<String, bool>,
}

impl Manifest {
    pub fn read(project_dir: &Path) -> Result<Option<Self>> {
        let path = project_dir.join(".claude").join("ccpick.json");
        let contents = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Ok(None);
            }
            Err(e) => {
                return Err(anyhow::Error::new(e).context(format!("reading {}", path.display())));
            }
        };
        let manifest: Self = serde_json::from_str(&contents)
            .with_context(|| format!("parsing {}", path.display()))?;
        Ok(Some(manifest))
    }

    pub fn write(&self, project_dir: &Path) -> Result<()> {
        let claude_dir = project_dir.join(".claude");
        fs::create_dir_all(&claude_dir)
            .with_context(|| format!("creating {}", claude_dir.display()))?;
        let path = claude_dir.join("ccpick.json");
        let json = serde_json::to_string_pretty(self).context("serializing manifest")?;
        fs::write(&path, json).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    pub fn read_template(templates_dir: &Path, name: &str) -> Result<Self> {
        let path = templates_dir.join(format!("{name}.json"));
        let contents =
            fs::read_to_string(&path).with_context(|| format!("template '{name}' not found"))?;
        let manifest: Self = serde_json::from_str(&contents)
            .with_context(|| format!("parsing template '{name}'"))?;
        Ok(manifest)
    }

    pub fn write_template(&self, templates_dir: &Path, name: &str) -> Result<()> {
        fs::create_dir_all(templates_dir)
            .with_context(|| format!("creating {}", templates_dir.display()))?;
        let path = templates_dir.join(format!("{name}.json"));
        let json = serde_json::to_string_pretty(self).context("serializing template")?;
        fs::write(&path, json).with_context(|| format!("writing template '{name}'"))?;
        Ok(())
    }

    pub fn list_templates(templates_dir: &Path) -> Result<Vec<String>> {
        if !templates_dir.is_dir() {
            return Ok(Vec::new());
        }
        let mut names = Vec::new();
        for entry in fs::read_dir(templates_dir)
            .with_context(|| format!("reading {}", templates_dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json")
                && let Some(stem) = path.file_stem()
            {
                names.push(stem.to_string_lossy().into_owned());
            }
        }
        names.sort();
        Ok(names)
    }

    pub fn delete_template(templates_dir: &Path, name: &str) -> Result<()> {
        let path = templates_dir.join(format!("{name}.json"));
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                anyhow::bail!("template '{name}' not found");
            }
            Err(e) => Err(anyhow::Error::new(e).context(format!("deleting template '{name}'"))),
        }
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used)]

    use super::*;

    fn sample_manifest() -> Manifest {
        let mut plugins = BTreeMap::new();
        plugins.insert("plugin-a".to_string(), true);
        plugins.insert("plugin-b".to_string(), false);
        Manifest {
            version: 1,
            commands: vec!["trailofbits/config.md".to_string()],
            skills: vec!["review.md".to_string()],
            plugins,
        }
    }

    #[test]
    fn roundtrip_write_then_read() {
        let tmp = tempfile::tempdir().unwrap();
        let manifest = sample_manifest();
        manifest.write(tmp.path()).unwrap();

        let loaded = Manifest::read(tmp.path()).unwrap().unwrap();
        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.commands, vec!["trailofbits/config.md"]);
        assert_eq!(loaded.skills, vec!["review.md"]);
        assert_eq!(loaded.plugins.get("plugin-a"), Some(&true));
        assert_eq!(loaded.plugins.get("plugin-b"), Some(&false));
    }

    #[test]
    fn read_missing_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let result = Manifest::read(tmp.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn read_malformed_json_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let claude_dir = tmp.path().join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();
        fs::write(claude_dir.join("ccpick.json"), "{not valid json").unwrap();

        let result = Manifest::read(tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn read_with_missing_optional_fields() {
        let tmp = tempfile::tempdir().unwrap();
        let claude_dir = tmp.path().join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();
        fs::write(claude_dir.join("ccpick.json"), r#"{"version": 1}"#).unwrap();

        let loaded = Manifest::read(tmp.path()).unwrap().unwrap();
        assert_eq!(loaded.version, 1);
        assert!(loaded.commands.is_empty());
        assert!(loaded.skills.is_empty());
        assert!(loaded.plugins.is_empty());
    }

    #[test]
    fn template_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let tpl_dir = tmp.path().join("templates");
        let manifest = sample_manifest();
        manifest.write_template(&tpl_dir, "backend").unwrap();

        let loaded = Manifest::read_template(&tpl_dir, "backend").unwrap();
        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.commands, vec!["trailofbits/config.md"]);
        assert_eq!(loaded.skills, vec!["review.md"]);
    }

    #[test]
    fn read_template_missing_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let result = Manifest::read_template(tmp.path(), "nope");
        assert!(result.is_err());
    }

    #[test]
    fn list_templates_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let names = Manifest::list_templates(tmp.path()).unwrap();
        assert!(names.is_empty());
    }

    #[test]
    fn list_templates_nonexistent_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("nope");
        let names = Manifest::list_templates(&missing).unwrap();
        assert!(names.is_empty());
    }

    #[test]
    fn list_templates_returns_sorted_names() {
        let tmp = tempfile::tempdir().unwrap();
        let tpl_dir = tmp.path().join("templates");
        fs::create_dir_all(&tpl_dir).unwrap();
        fs::write(tpl_dir.join("zebra.json"), "{}").unwrap();
        fs::write(tpl_dir.join("alpha.json"), "{}").unwrap();
        fs::write(tpl_dir.join("not-json.txt"), "ignored").unwrap();

        let names = Manifest::list_templates(&tpl_dir).unwrap();
        assert_eq!(names, vec!["alpha", "zebra"]);
    }

    #[test]
    fn delete_template_removes_file() {
        let tmp = tempfile::tempdir().unwrap();
        let tpl_dir = tmp.path().join("templates");
        let manifest = sample_manifest();
        manifest.write_template(&tpl_dir, "doomed").unwrap();
        assert!(tpl_dir.join("doomed.json").exists());

        Manifest::delete_template(&tpl_dir, "doomed").unwrap();
        assert!(!tpl_dir.join("doomed.json").exists());
    }

    #[test]
    fn delete_template_missing_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let result = Manifest::delete_template(tmp.path(), "ghost");
        assert!(result.is_err());
    }
}
