use std::path::PathBuf;

use anyhow::{Context, Result};

pub struct Paths {
    /// `~/.claude`
    pub claude_home: PathBuf,
    /// `~/.claude/ccpick/{commands,skills}`
    pub library: PathBuf,
    /// `~/.claude/commands`
    pub global_commands: PathBuf,
    /// `~/.claude/skills`
    pub global_skills: PathBuf,
}

impl Paths {
    pub fn resolve() -> Result<Self> {
        let home = dirs::home_dir()
            .context("could not determine home directory")?;
        let claude_home = home.join(".claude");
        let library = claude_home.join("ccpick");
        let global_commands = claude_home.join("commands");
        let global_skills = claude_home.join("skills");
        Ok(Self {
            claude_home,
            library,
            global_commands,
            global_skills,
        })
    }

    pub fn library_commands(&self) -> PathBuf {
        self.library.join("commands")
    }

    pub fn library_skills(&self) -> PathBuf {
        self.library.join("skills")
    }

    pub fn templates_dir(&self) -> PathBuf {
        self.library.join("templates")
    }
}
