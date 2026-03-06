use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::Value;

use crate::config::Paths;

/// Read `enabledPlugins` from `~/.claude/settings.json`.
pub fn read_global_enabled_plugins(paths: &Paths) -> Result<BTreeMap<String, bool>> {
    let path = paths.claude_home.join("settings.json");
    let contents = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Ok(BTreeMap::new());
        }
        Err(e) => {
            return Err(anyhow::Error::new(e).context(format!("reading {}", path.display())));
        }
    };
    let val: Value =
        serde_json::from_str(&contents).with_context(|| format!("parsing {}", path.display()))?;
    let Some(obj) = val.get("enabledPlugins").and_then(Value::as_object) else {
        return Ok(BTreeMap::new());
    };
    let mut result = BTreeMap::new();
    for (k, v) in obj {
        if let Some(b) = v.as_bool() {
            result.insert(k.clone(), b);
        }
    }
    Ok(result)
}

/// Merge `enabledPlugins` into a settings JSON file, preserving all other
/// keys. Works for both global `~/.claude/settings.json` and project-level
/// `.claude/settings.local.json`.
pub fn merge_enabled_plugins(settings_path: &Path, plugins: &BTreeMap<String, bool>) -> Result<()> {
    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }

    let mut settings: Value = if settings_path.exists() {
        let contents = fs::read_to_string(settings_path)
            .with_context(|| format!("reading {}", settings_path.display()))?;
        serde_json::from_str(&contents)
            .with_context(|| format!("parsing {}", settings_path.display()))?
    } else {
        Value::Object(serde_json::Map::new())
    };

    let obj = settings
        .as_object_mut()
        .with_context(|| format!("{} is not a JSON object", settings_path.display()))?;

    let plugin_map: serde_json::Map<String, Value> = plugins
        .iter()
        .map(|(k, v)| (k.clone(), Value::Bool(*v)))
        .collect();

    obj.insert("enabledPlugins".to_string(), Value::Object(plugin_map));

    let json = serde_json::to_string_pretty(&settings).context("serializing settings")?;
    fs::write(settings_path, json)
        .with_context(|| format!("writing {}", settings_path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn merge_into_new_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.local.json");

        let mut plugins = BTreeMap::new();
        plugins.insert("plugin-a".to_string(), true);
        plugins.insert("plugin-b".to_string(), false);

        merge_enabled_plugins(&path, &plugins).unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        let val: Value = serde_json::from_str(&contents).unwrap();
        let ep = val.get("enabledPlugins").unwrap();
        assert_eq!(ep.get("plugin-a").unwrap(), true);
        assert_eq!(ep.get("plugin-b").unwrap(), false);
    }

    #[test]
    fn merge_preserves_existing_keys() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        fs::write(
            &path,
            r#"{"env":{"FOO":"bar"},"enabledPlugins":{"old":true}}"#,
        )
        .unwrap();

        let mut plugins = BTreeMap::new();
        plugins.insert("new-plugin".to_string(), true);

        merge_enabled_plugins(&path, &plugins).unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        let val: Value = serde_json::from_str(&contents).unwrap();
        assert_eq!(
            val.get("env")
                .unwrap()
                .get("FOO")
                .unwrap()
                .as_str()
                .unwrap(),
            "bar",
        );
        let ep = val.get("enabledPlugins").unwrap();
        assert_eq!(ep.get("new-plugin").unwrap(), true);
        assert!(ep.get("old").is_none(), "old plugins replaced, not merged");
    }

    #[test]
    fn merge_errors_on_non_object() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        fs::write(&path, r#""just a string""#).unwrap();

        let plugins = BTreeMap::new();
        let result = merge_enabled_plugins(&path, &plugins);
        assert!(result.is_err());
    }

    #[test]
    fn merge_creates_parent_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("deep").join("settings.json");

        let plugins = BTreeMap::new();
        merge_enabled_plugins(&path, &plugins).unwrap();
        assert!(path.exists());
    }
}
