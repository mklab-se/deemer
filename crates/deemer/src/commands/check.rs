//! `deemer check` — validate a suite without running it.

use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;

/// Load and statically validate a suite. Exits with code 2 if any issues are found.
pub async fn run(path: PathBuf) -> Result<()> {
    let suite = deemer_core::suite::Suite::load(&path)?;
    let issues = deemer_core::check::check(&suite);

    if issues.is_empty() {
        println!(
            "{} {} ({} tests)",
            "OK:".green().bold(),
            suite.name,
            suite.data.len()
        );
        Ok(())
    } else {
        for issue in &issues {
            eprintln!("{} {issue}", "✗".red());
        }
        eprintln!("\n{} issue(s) found", issues.len());
        std::process::exit(2);
    }
}
