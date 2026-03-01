use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "ccpick",
    about = "Per-project Claude Code extension manager"
)]
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
    /// Manage reusable configuration templates
    Template(TemplateArgs),
}

#[derive(Args)]
pub struct InitArgs {
    /// Apply a saved template as pre-checked defaults
    #[arg(long)]
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
        name: String,
    },
    /// Interactively update an existing template
    Edit {
        /// Template name
        name: String,
    },
    /// List saved templates
    List,
    /// Delete a saved template
    Delete {
        /// Template name
        name: String,
    },
}
