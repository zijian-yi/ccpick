use clap::{Args, Parser, Subcommand};
use clap_complete::engine::ArgValueCandidates;

use crate::completions;

#[derive(Parser)]
#[command(name = "ccpick", about = "Per-project Claude Code extension manager")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Move selected global commands/skills into the ccpick library
    Tidy(TidyArgs),
    /// Interactively pick which commands, skills, and plugins to enable
    Init(InitArgs),
    /// Interactively update selections in an existing ccpick.json
    Edit,
    /// Re-apply selections from the saved manifest
    Sync,
    /// Install commands/skills from a GitHub repository
    Install(InstallArgs),
    /// Manage reusable configuration templates
    Template(TemplateArgs),
    /// Manage CLAUDE.md/AGENTS.md templates and presets
    Guide(GuideArgs),
}

#[derive(Args)]
pub struct InstallArgs {
    /// GitHub repository URL (e.g. owner/repo or full URL)
    pub url: String,
    /// Install to ~/.claude/ (global)
    #[arg(short, long, conflicts_with = "local")]
    pub global: bool,
    /// Install to .claude/ (current project only)
    #[arg(short, long, conflicts_with = "global")]
    pub local: bool,
    /// Git branch (overrides branch parsed from URL)
    #[arg(long)]
    pub branch: Option<String>,
}

#[derive(Args)]
pub struct InitArgs {
    /// Apply a saved template as pre-checked defaults
    #[arg(long, add = ArgValueCandidates::new(completions::manifest_templates))]
    pub template: Option<String>,
}

#[derive(Args)]
pub struct TidyArgs {
    /// Only tidy commands
    #[arg(long)]
    pub commands: bool,
    /// Only tidy skills
    #[arg(long)]
    pub skills: bool,
    /// Only tidy plugins
    #[arg(long)]
    pub plugins: bool,
}

impl TidyArgs {
    pub fn want_commands(&self) -> bool {
        self.all() || self.commands
    }

    pub fn want_skills(&self) -> bool {
        self.all() || self.skills
    }

    pub fn want_plugins(&self) -> bool {
        self.all() || self.plugins
    }

    fn all(&self) -> bool {
        !self.commands && !self.skills && !self.plugins
    }
}

#[derive(Args)]
pub struct GuideArgs {
    #[command(subcommand)]
    pub action: GuideAction,
}

#[derive(Subcommand)]
pub enum GuideAction {
    /// Manage guide templates
    Template {
        #[command(subcommand)]
        action: GuideTemplateAction,
    },
    /// Manage guide presets
    Preset {
        #[command(subcommand)]
        action: GuidePresetAction,
    },
    /// Apply a top-level preset to write CLAUDE.md + AGENTS.md
    Apply {
        /// Preset name (opens picker if omitted)
        #[arg(add = ArgValueCandidates::new(completions::guide_presets))]
        name: Option<String>,
    },
    /// Compose from a template by filling slots interactively
    Compose {
        /// Template name (opens picker if omitted)
        #[arg(add = ArgValueCandidates::new(completions::guide_templates))]
        name: Option<String>,
    },
    /// Preview rendered output without writing files
    Show {
        /// Preset or template name
        #[arg(add = ArgValueCandidates::new(completions::guide_presets_and_templates))]
        name: String,
    },
}

#[derive(Subcommand)]
pub enum GuideTemplateAction {
    /// List guide templates
    List,
    /// Create a new guide template in $EDITOR
    Create {
        /// Template name
        name: String,
    },
    /// Edit an existing guide template in $EDITOR
    Edit {
        /// Template name
        #[arg(add = ArgValueCandidates::new(completions::guide_templates))]
        name: String,
    },
    /// Delete a guide template
    Delete {
        /// Template name
        #[arg(add = ArgValueCandidates::new(completions::guide_templates))]
        name: String,
    },
}

#[derive(Subcommand)]
pub enum GuidePresetAction {
    /// List presets
    List {
        /// Slot name (omit for top-level presets)
        #[arg(add = ArgValueCandidates::new(completions::guide_preset_slots))]
        slot: Option<String>,
    },
    /// Create a preset
    Create(GuidePresetCreateArgs),
    /// Edit a preset in $EDITOR
    Edit {
        /// Preset name (top-level) or slot name (with second arg)
        #[arg(add = ArgValueCandidates::new(completions::guide_presets_and_slots))]
        name: String,
        /// Preset name within the slot
        slot_preset: Option<String>,
    },
    /// Delete a preset
    Delete {
        /// Preset name (top-level) or slot name (with second arg)
        #[arg(add = ArgValueCandidates::new(completions::guide_presets_and_slots))]
        name: String,
        /// Preset name within the slot
        slot_preset: Option<String>,
    },
}

#[derive(Args)]
pub struct GuidePresetCreateArgs {
    /// Preset name (top-level) or slot name (with second arg)
    pub name: String,
    /// Preset name within the slot
    pub slot_preset: Option<String>,
    /// Build top-level preset from a template
    #[arg(long, add = ArgValueCandidates::new(completions::guide_templates))]
    pub from_template: Option<String>,
}

#[derive(Args)]
pub struct TemplateArgs {
    #[command(subcommand)]
    pub action: TemplateAction,
}

#[derive(Subcommand)]
pub enum TemplateAction {
    /// Save the current project manifest as a named template
    Save {
        /// Template name
        name: String,
    },
    /// Interactively create a new template
    Create {
        /// Template name (prompted if omitted)
        name: Option<String>,
    },
    /// Apply a template to the current project (shortcut for `init --template`)
    Apply {
        /// Template name
        #[arg(add = ArgValueCandidates::new(completions::manifest_templates))]
        name: String,
    },
    /// Interactively update an existing template
    Edit {
        /// Template name
        #[arg(add = ArgValueCandidates::new(completions::manifest_templates))]
        name: String,
    },
    /// List saved templates
    List,
    /// Delete a saved template
    Delete {
        /// Template name
        #[arg(add = ArgValueCandidates::new(completions::manifest_templates))]
        name: String,
    },
}
