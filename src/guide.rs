use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

// ---------------------------------------------------------------------------
// Template parsing and rendering
// ---------------------------------------------------------------------------

pub fn parse_slots(template: &str) -> Result<Vec<String>> {
    let mut slots = Vec::new();
    for line in template.lines() {
        let trimmed = line.trim();
        if let Some(inner) = strip_placeholder(trimmed) {
            let name = inner.trim();
            validate_slot_name(name)?;
            if slots.contains(&name.to_string()) {
                bail!("duplicate slot: {name}");
            }
            slots.push(name.to_string());
        }
    }
    Ok(slots)
}

pub fn render(template: &str, fills: &BTreeMap<String, String>) -> Result<String> {
    let slots = parse_slots(template)?;
    for slot in &slots {
        if !fills.contains_key(slot) {
            bail!("unfilled slot: {slot}");
        }
    }

    let mut output = String::new();
    for line in template.lines() {
        let trimmed = line.trim();
        if let Some(inner) = strip_placeholder(trimmed) {
            let name = inner.trim();
            if let Some(content) = fills.get(name) {
                output.push_str(content);
                output.push('\n');
            }
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }

    // Remove trailing newline added by loop if original doesn't end with one
    if !template.ends_with('\n') && output.ends_with('\n') {
        output.pop();
    }

    Ok(output)
}

fn strip_placeholder(s: &str) -> Option<&str> {
    let s = s.strip_prefix("{{")?;
    let s = s.strip_suffix("}}")?;
    Some(s)
}

fn validate_slot_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("slot name cannot be empty");
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        bail!(
            "slot name '{name}' may only contain \
             ASCII alphanumeric characters and underscores"
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Name validation
// ---------------------------------------------------------------------------

pub fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("name cannot be empty");
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        bail!(
            "name '{name}' may only contain \
             alphanumeric characters, hyphens, and underscores"
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Directory helpers
// ---------------------------------------------------------------------------

fn templates_dir(guide_dir: &Path) -> PathBuf {
    guide_dir.join("templates")
}

fn presets_dir(guide_dir: &Path) -> PathBuf {
    guide_dir.join("presets")
}

fn slot_presets_dir(guide_dir: &Path, slot: &str) -> PathBuf {
    presets_dir(guide_dir).join(slot)
}

// ---------------------------------------------------------------------------
// Template CRUD
// ---------------------------------------------------------------------------

pub fn list_templates(guide_dir: &Path) -> Result<Vec<String>> {
    list_md_files(&templates_dir(guide_dir))
}

pub fn read_template(guide_dir: &Path, name: &str) -> Result<String> {
    let path = templates_dir(guide_dir).join(format!("{name}.md"));
    fs::read_to_string(&path).with_context(|| format!("template '{name}' not found"))
}

pub fn write_template(guide_dir: &Path, name: &str, content: &str) -> Result<()> {
    let dir = templates_dir(guide_dir);
    fs::create_dir_all(&dir)?;
    fs::write(dir.join(format!("{name}.md")), content)?;
    Ok(())
}

pub fn delete_template(guide_dir: &Path, name: &str) -> Result<()> {
    let path = templates_dir(guide_dir).join(format!("{name}.md"));
    fs::remove_file(&path).with_context(|| format!("template '{name}' not found"))
}

// ---------------------------------------------------------------------------
// Top-level preset CRUD
// ---------------------------------------------------------------------------

pub fn list_presets(guide_dir: &Path) -> Result<Vec<String>> {
    list_md_files(&presets_dir(guide_dir))
}

pub fn read_preset(guide_dir: &Path, name: &str) -> Result<String> {
    let path = presets_dir(guide_dir).join(format!("{name}.md"));
    fs::read_to_string(&path).with_context(|| format!("preset '{name}' not found"))
}

pub fn write_preset(guide_dir: &Path, name: &str, content: &str) -> Result<()> {
    let dir = presets_dir(guide_dir);
    fs::create_dir_all(&dir)?;
    fs::write(dir.join(format!("{name}.md")), content)?;
    Ok(())
}

pub fn delete_preset(guide_dir: &Path, name: &str) -> Result<()> {
    let path = presets_dir(guide_dir).join(format!("{name}.md"));
    fs::remove_file(&path).with_context(|| format!("preset '{name}' not found"))
}

// ---------------------------------------------------------------------------
// Slot preset CRUD
// ---------------------------------------------------------------------------

pub fn list_preset_slots(guide_dir: &Path) -> Result<Vec<String>> {
    let dir = presets_dir(guide_dir);
    let mut slots = Vec::new();
    let Ok(entries) = fs::read_dir(&dir) else {
        return Ok(slots);
    };
    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_dir()
            && let Some(name) = entry.file_name().to_str()
        {
            slots.push(name.to_string());
        }
    }
    slots.sort();
    Ok(slots)
}

pub fn list_slot_presets(guide_dir: &Path, slot: &str) -> Result<Vec<String>> {
    list_md_files(&slot_presets_dir(guide_dir, slot))
}

pub fn read_slot_preset(guide_dir: &Path, slot: &str, name: &str) -> Result<String> {
    let path = slot_presets_dir(guide_dir, slot).join(format!("{name}.md"));
    fs::read_to_string(&path).with_context(|| format!("slot preset '{slot}/{name}' not found"))
}

pub fn write_slot_preset(guide_dir: &Path, slot: &str, name: &str, content: &str) -> Result<()> {
    let dir = slot_presets_dir(guide_dir, slot);
    fs::create_dir_all(&dir)?;
    fs::write(dir.join(format!("{name}.md")), content)?;
    Ok(())
}

pub fn delete_slot_preset(guide_dir: &Path, slot: &str, name: &str) -> Result<()> {
    let path = slot_presets_dir(guide_dir, slot).join(format!("{name}.md"));
    fs::remove_file(&path).with_context(|| format!("slot preset '{slot}/{name}' not found"))
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn list_md_files(dir: &Path) -> Result<Vec<String>> {
    let mut names = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(names);
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("md")
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
        {
            names.push(stem.to_string());
        }
    }
    names.sort();
    Ok(names)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn parse_slots_extracts_names() {
        let template = "# Title\n{{ language }}\nsome text\n{{ testing }}\n";
        let slots = parse_slots(template).unwrap();
        assert_eq!(slots, vec!["language", "testing"]);
    }

    #[test]
    fn parse_slots_rejects_duplicates() {
        let template = "{{ foo }}\n{{ foo }}\n";
        let err = parse_slots(template).unwrap_err();
        assert!(err.to_string().contains("duplicate slot"));
    }

    #[test]
    fn parse_slots_rejects_invalid_names() {
        let template = "{{ foo bar }}\n";
        let err = parse_slots(template).unwrap_err();
        assert!(err.to_string().contains("may only contain"));
    }

    #[test]
    fn parse_slots_empty_template() {
        let slots = parse_slots("no placeholders here").unwrap();
        assert!(slots.is_empty());
    }

    #[test]
    fn render_fills_slots() {
        let template = "# Header\n{{ language }}\n## Footer\n";
        let mut fills = BTreeMap::new();
        fills.insert("language".to_string(), "Use Rust.".to_string());
        let result = render(template, &fills).unwrap();
        assert_eq!(result, "# Header\nUse Rust.\n## Footer\n");
    }

    #[test]
    fn render_errors_on_unfilled() {
        let template = "{{ language }}\n";
        let fills = BTreeMap::new();
        let err = render(template, &fills).unwrap_err();
        assert!(err.to_string().contains("unfilled slot"));
    }

    #[test]
    fn render_multiline_fill() {
        let template = "before\n{{ section }}\nafter\n";
        let mut fills = BTreeMap::new();
        fills.insert("section".to_string(), "line1\nline2".to_string());
        let result = render(template, &fills).unwrap();
        assert_eq!(result, "before\nline1\nline2\nafter\n");
    }

    #[test]
    fn strip_placeholder_works() {
        assert_eq!(strip_placeholder("{{ foo }}"), Some(" foo "));
        assert_eq!(strip_placeholder("{{foo}}"), Some("foo"));
        assert_eq!(strip_placeholder("not a slot"), None);
        assert_eq!(strip_placeholder("{{ foo"), None);
    }

    #[test]
    fn validate_name_accepts_valid() {
        assert!(validate_name("rust-backend").is_ok());
        assert!(validate_name("my_preset_1").is_ok());
    }

    #[test]
    fn validate_name_rejects_invalid() {
        assert!(validate_name("").is_err());
        assert!(validate_name("has spaces").is_err());
        assert!(validate_name("path/sep").is_err());
    }

    #[test]
    fn template_crud_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let guide_dir = dir.path();

        assert!(list_templates(guide_dir).unwrap().is_empty());

        write_template(guide_dir, "base", "# Base\n{{ lang }}\n").unwrap();
        assert_eq!(list_templates(guide_dir).unwrap(), vec!["base"]);

        let content = read_template(guide_dir, "base").unwrap();
        assert!(content.contains("{{ lang }}"));

        delete_template(guide_dir, "base").unwrap();
        assert!(list_templates(guide_dir).unwrap().is_empty());
    }

    #[test]
    fn preset_crud_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let guide_dir = dir.path();

        write_preset(guide_dir, "rust-backend", "# Rust Backend").unwrap();
        assert_eq!(list_presets(guide_dir).unwrap(), vec!["rust-backend"]);

        let content = read_preset(guide_dir, "rust-backend").unwrap();
        assert_eq!(content, "# Rust Backend");

        delete_preset(guide_dir, "rust-backend").unwrap();
        assert!(list_presets(guide_dir).unwrap().is_empty());
    }

    #[test]
    fn slot_preset_crud_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let guide_dir = dir.path();

        write_slot_preset(guide_dir, "language", "rust", "Use Rust").unwrap();
        assert_eq!(list_preset_slots(guide_dir).unwrap(), vec!["language"]);
        assert_eq!(
            list_slot_presets(guide_dir, "language").unwrap(),
            vec!["rust"]
        );

        let content = read_slot_preset(guide_dir, "language", "rust").unwrap();
        assert_eq!(content, "Use Rust");

        delete_slot_preset(guide_dir, "language", "rust").unwrap();
        assert!(list_slot_presets(guide_dir, "language").unwrap().is_empty());
    }
}
