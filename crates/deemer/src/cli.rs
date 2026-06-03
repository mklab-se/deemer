//! CLI argument definitions using clap.

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

/// Run AI-assisted integration tests that judge whether your tests passed.
#[derive(Parser)]
#[command(name = "deemer")]
#[command(author, version, about)]
#[command(
    long_about = "deemer runs AI-assisted integration tests — when deciding whether a \
    test or test suite worked needs AI judgement rather than a plain assertion.\n\n\
    Run a suite with `deemer run <suite.yml>`, or validate one with `deemer check \
    <suite.yml>`. See the docs and samples for the suite format."
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

    /// Run a test suite and write its results log + report
    Run {
        /// Path to the suite file (e.g. checks.suite.yml)
        suite: PathBuf,
        /// Override the report output path for this run
        #[arg(long)]
        report: Option<PathBuf>,
        /// Override the results-log output path for this run
        #[arg(long)]
        results: Option<PathBuf>,
        /// Override the suite's concurrency for this run
        #[arg(long)]
        concurrency: Option<usize>,
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
            Some(Commands::Run {
                suite,
                report,
                results,
                concurrency,
            }) => crate::commands::run::run(suite, report, results, concurrency).await,
            Some(Commands::Check { suite }) => crate::commands::check::run(suite).await,
            Some(Commands::Version) => {
                crate::banner::print_banner_with_version();
                Ok(())
            }
            // No subcommand: show the banner and point at the real commands.
            // `--help` still works via clap.
            None => {
                crate::banner::print_banner_with_version();
                println!("Run a suite with:  deemer run <suite.yml>");
                println!("Validate one with: deemer check <suite.yml>");
                println!("More:              deemer --help");
                Ok(())
            }
        }
    }
}
