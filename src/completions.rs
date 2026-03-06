use clap_complete::engine::CompletionCandidate;

use crate::config::Paths;
use crate::guide;
use crate::manifest::Manifest;

fn guide_dir() -> Option<std::path::PathBuf> {
    Paths::resolve().ok().map(|p| p.guide_dir())
}

fn templates_dir() -> Option<std::path::PathBuf> {
    Paths::resolve().ok().map(|p| p.templates_dir())
}

fn to_candidates(names: Vec<String>) -> Vec<CompletionCandidate> {
    let mut candidates = Vec::with_capacity(names.len());
    for name in names {
        candidates.push(CompletionCandidate::new(name));
    }
    candidates
}

pub fn guide_templates() -> Vec<CompletionCandidate> {
    let Some(dir) = guide_dir() else {
        return Vec::new();
    };
    let Ok(names) = guide::list_templates(&dir) else {
        return Vec::new();
    };
    to_candidates(names)
}

pub fn guide_presets() -> Vec<CompletionCandidate> {
    let Some(dir) = guide_dir() else {
        return Vec::new();
    };
    let Ok(names) = guide::list_presets(&dir) else {
        return Vec::new();
    };
    to_candidates(names)
}

pub fn guide_presets_and_templates() -> Vec<CompletionCandidate> {
    let Some(dir) = guide_dir() else {
        return Vec::new();
    };
    let mut names = guide::list_presets(&dir).unwrap_or_default();
    names.extend(
        guide::list_templates(&dir).unwrap_or_default(),
    );
    names.sort();
    names.dedup();
    to_candidates(names)
}

pub fn guide_preset_slots() -> Vec<CompletionCandidate> {
    let Some(dir) = guide_dir() else {
        return Vec::new();
    };
    let Ok(names) = guide::list_preset_slots(&dir) else {
        return Vec::new();
    };
    to_candidates(names)
}

pub fn guide_presets_and_slots() -> Vec<CompletionCandidate> {
    let Some(dir) = guide_dir() else {
        return Vec::new();
    };
    let mut names = guide::list_presets(&dir).unwrap_or_default();
    names.extend(
        guide::list_preset_slots(&dir).unwrap_or_default(),
    );
    names.sort();
    names.dedup();
    to_candidates(names)
}

pub fn manifest_templates() -> Vec<CompletionCandidate> {
    let Some(dir) = templates_dir() else {
        return Vec::new();
    };
    let Ok(names) = Manifest::list_templates(&dir) else {
        return Vec::new();
    };
    to_candidates(names)
}
