use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct InstalledPlugins {
    plugins: BTreeMap<String, Vec<PluginInstall>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginInstall {
    install_path: String,
}

#[derive(Debug, Deserialize)]
struct PluginMeta {
    name: String,
    #[serde(default)]
    description: String,
}

#[derive(Debug)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub description: String,
}

impl PluginInfo {
    pub fn label(&self) -> String {
        if self.description.is_empty() {
            self.name.clone()
        } else {
            format!("{} — {}", self.name, self.description)
        }
    }
}

pub fn scan_plugins(claude_home: &Path) -> Result<Vec<PluginInfo>> {
    let path = claude_home
        .join("plugins")
        .join("installed_plugins.json");

    if !path.exists() {
        return Ok(Vec::new());
    }

    let contents = fs::read_to_string(&path)
        .with_context(|| {
            format!("reading {}", path.display())
        })?;
    let installed: InstalledPlugins =
        serde_json::from_str(&contents).with_context(|| {
            format!("parsing {}", path.display())
        })?;

    let mut plugins = Vec::new();
    for (id, installs) in &installed.plugins {
        let Some(install) = installs.first() else {
            continue;
        };
        let meta_path = Path::new(&install.install_path)
            .join(".claude-plugin")
            .join("plugin.json");

        let (name, description) = if meta_path.exists() {
            let meta_str = fs::read_to_string(&meta_path)
                .with_context(|| {
                    format!("reading {}", meta_path.display())
                })?;
            let meta: PluginMeta =
                serde_json::from_str(&meta_str).with_context(|| {
                    format!("parsing {}", meta_path.display())
                })?;
            (meta.name, meta.description)
        } else {
            (id.clone(), String::new())
        };

        plugins.push(PluginInfo {
            id: id.clone(),
            name,
            description,
        });
    }

    plugins.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(plugins)
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used)]

    use super::*;

    fn write_installed_plugins(
        claude_home: &Path,
        json: &str,
    ) {
        let dir = claude_home.join("plugins");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("installed_plugins.json"), json)
            .unwrap();
    }

    fn write_plugin_meta(
        install_path: &Path,
        name: &str,
        desc: &str,
    ) {
        let meta_dir = install_path.join(".claude-plugin");
        fs::create_dir_all(&meta_dir).unwrap();
        fs::write(
            meta_dir.join("plugin.json"),
            format!(
                r#"{{"name":"{name}","description":"{desc}"}}"#
            ),
        )
        .unwrap();
    }

    #[test]
    fn scans_plugins_with_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        let claude_home = tmp.path();

        let install_dir = tmp.path().join("installs/cool");
        write_plugin_meta(
            &install_dir,
            "Cool Plugin",
            "Does cool things",
        );

        let json = format!(
            r#"{{"plugins":{{"acme/cool-plugin":[{{"installPath":"{}"}}]}}}}"#,
            install_dir.display(),
        );
        write_installed_plugins(claude_home, &json);

        let result = scan_plugins(claude_home).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "acme/cool-plugin");
        assert_eq!(result[0].name, "Cool Plugin");
        assert_eq!(result[0].description, "Does cool things");
    }

    #[test]
    fn falls_back_to_id_when_meta_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let claude_home = tmp.path();

        let json = r#"{"plugins":{"some/plugin":[{"installPath":"/nonexistent"}]}}"#;
        write_installed_plugins(claude_home, json);

        let result = scan_plugins(claude_home).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "some/plugin");
        assert_eq!(result[0].name, "some/plugin");
        assert!(result[0].description.is_empty());
    }

    #[test]
    fn missing_file_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let result = scan_plugins(tmp.path()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn skips_plugins_with_no_installs() {
        let tmp = tempfile::tempdir().unwrap();
        let json = r#"{"plugins":{"empty/plugin":[]}}"#;
        write_installed_plugins(tmp.path(), json);

        let result = scan_plugins(tmp.path()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn sorts_by_id() {
        let tmp = tempfile::tempdir().unwrap();
        let json = r#"{"plugins":{"z/last":[{"installPath":"/nope"}],"a/first":[{"installPath":"/nope"}]}}"#;
        write_installed_plugins(tmp.path(), json);

        let result = scan_plugins(tmp.path()).unwrap();
        assert_eq!(result[0].id, "a/first");
        assert_eq!(result[1].id, "z/last");
    }
}
