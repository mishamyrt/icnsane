#[cfg(not(target_os = "macos"))]
compile_error!("icnsane requires macOS");

mod config;
mod macos;

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::ExitCode,
};

use anyhow::{Result, ensure};
use clap::{Parser, Subcommand};
use objc2::rc::autoreleasepool;

use config::Config;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Path to the TOML configuration.
    #[arg(long, global = true, default_value = "icnsane.toml")]
    config: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Apply the icons listed in the configuration.
    Apply {
        /// Validate the applications and images without changing icons.
        #[arg(long)]
        dry_run: bool,
    },
    /// Restore the default icon of an application.
    Reset {
        /// Absolute application path (also accepts ~/).
        #[arg(required_unless_present = "all")]
        app: Option<PathBuf>,

        /// Reset all applications listed in the configuration.
        #[arg(long, conflicts_with = "app")]
        all: bool,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<bool> {
    if let Command::Reset { app: Some(app), .. } = &cli.command {
        let result = autoreleasepool(|_| config::app_path(app).and_then(|path| reset(&path)));
        return Ok(report(app, result));
    }

    let config = Config::load(&cli.config)?;
    let apply_options = match cli.command {
        Command::Apply { dry_run } => Some((config.icons_dir(&cli.config)?, dry_run)),
        Command::Reset { .. } => None,
    };
    let mut applications = BTreeMap::new();
    let mut success = true;

    for (app, icon) in &config.apps {
        match config::app_path(app) {
            Ok(resolved) => {
                applications
                    .entry(resolved)
                    .and_modify(|stored| *stored = None)
                    .or_insert(Some(icon));
            }
            Err(error) => success &= report(app, Err(error)),
        }
    }

    for (app, icon) in applications {
        let result = autoreleasepool(|_| match (icon, &apply_options) {
            (None, _) => {
                anyhow::bail!("multiple configuration entries resolve to this application")
            }
            (Some(icon), Some((dir, dry_run))) => apply(&app, dir, icon, *dry_run),
            (Some(_), None) => reset(&app),
        });
        success &= report(&app, result);
    }

    Ok(success)
}

fn apply(app: &Path, dir: &Path, icon: &Path, dry_run: bool) -> Result<String> {
    ensure!(
        icon.is_relative() && !icon.as_os_str().is_empty(),
        "icon path must be relative to icons_dir"
    );
    let icon_path = dir.join(icon);
    let image = macos::load_icon(&icon_path)?;
    let status = if dry_run {
        "would apply"
    } else {
        macos::set_icon(app, Some(&image))?;
        "applied"
    };
    Ok(format!("{status} {}", icon_path.display()))
}

fn reset(app: &Path) -> Result<String> {
    macos::set_icon(app, None)?;
    Ok("reset".into())
}

fn report(app: &Path, result: Result<String>) -> bool {
    match result {
        Ok(message) => {
            println!("{}: {message}", app.display());
            true
        }
        Err(error) => {
            eprintln!("{}: {error:#}", app.display());
            false
        }
    }
}
