//! CLI argument definitions using clap.

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

/// Run AI-assisted integration tests that judge whether your tests passed.
#[derive(Parser)]
#[command(name = "deemer")]
#[command(author, version, about)]
#[command(
    long_about = "deemer runs AI-assisted integration tests — when verifying \
    whether a test or test suite worked needs AI judgement rather than a simple assertion.\n\n\
    Run without a subcommand to print \"Hello world!\". The reusable plumbing — \
    AI integration, shell completions, and versioning — is wired up so you can \
    focus on deemer's own commands."
)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Increase output verbosity (-v for debug, -vv for trace)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Suppress non-essential output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Disable colored output
    #[arg(long, global = true)]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(clap::Subcommand)]
pub enum Commands {
    /// Manage AI features (shows status when run without a subcommand)
    Ai {
        #[command(subcommand)]
        command: Option<AiCommands>,
    },

    /// Generate shell completions
    Completion {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },

    /// Validate a test suite without running it
    Check {
        /// Path to the suite file (e.g. checks.suite.yml)
        suite: PathBuf,
    },

    /// Show version information
    Version,
}

#[derive(clap::Subcommand)]
pub enum AiCommands {
    /// Test AI integration by sending a message
    Test {
        /// Message to send (default: "Say hello in one sentence.")
        message: Option<String>,
    },
    /// Enable AI features for deemer
    Enable,
    /// Disable AI features for deemer
    Disable,
    /// Interactively configure AI provider and model settings
    Config,
    /// Show AI status (same as running `deemer ai` without a subcommand)
    Status,
}

/// Shells supported by `deemer completion`.
#[derive(Clone, Copy, clap::ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    Powershell,
}

impl Cli {
    /// Dispatch to the selected subcommand.
    pub async fn run(self) -> Result<()> {
        match self.command {
            Some(Commands::Ai { command }) => crate::commands::ai::run(command).await,
            Some(Commands::Completion { shell }) => {
                crate::commands::completion::generate_completions(shell);
                Ok(())
            }
            Some(Commands::Check { suite }) => crate::commands::check::run(suite).await,
            Some(Commands::Version) => {
                crate::banner::print_banner_with_version();
                Ok(())
            }
            // No subcommand: the almost-empty default. Replace this with your
            // tool's behavior, or route to a real command. `--help` still works.
            None => {
                println!("Hello world!");
                Ok(())
            }
        }
    }
}
