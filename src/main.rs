mod app_paths;
mod settings;

#[cfg(target_os = "macos")]
mod app_window;
#[cfg(target_os = "macos")]
mod desktop_ui;
#[cfg(target_os = "macos")]
mod login_item;
#[cfg(target_os = "macos")]
mod sparkle;

use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;

/// Display name used in menus and window titles. Keep in sync with
/// `scripts/app-config.sh`; a unit test enforces it.
pub const APP_NAME: &str = "Starter";
/// Bundle identifier. It also names the data directory.
pub const BUNDLE_ID: &str = "com.example.starter";

#[derive(Parser)]
#[command(name = "starter", version, about = "Rust desktop starter")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

/// A top-level pane of the main window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Pane {
    Home,
    Settings,
    About,
}

#[derive(Subcommand)]
enum Command {
    /// Open the desktop app. This is the default when no command is given.
    App {
        /// The pane to show first.
        #[arg(long, value_enum, default_value_t = Pane::Home)]
        pane: Pane,
    },
    /// Print the data, settings, and log locations.
    Paths,
    /// Print the saved settings as JSON.
    Settings,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let command = Cli::parse()
        .command
        .unwrap_or(Command::App { pane: Pane::Home });
    match command {
        Command::App { pane } => {
            app_paths::init_logging()?;
            run_app(pane)
        }
        Command::Paths => {
            println!("data:     {}", app_paths::support_dir()?.display());
            println!("settings: {}", app_paths::settings_path()?.display());
            println!("logs:     {}", app_paths::logs_dir()?.display());
            Ok(())
        }
        Command::Settings => {
            let settings = settings::Settings::load()?;
            println!("{}", serde_json::to_string_pretty(&settings)?);
            Ok(())
        }
    }
}

#[cfg(target_os = "macos")]
fn run_app(pane: Pane) -> Result<()> {
    app_window::run(pane);
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn run_app(_pane: Pane) -> Result<()> {
    Err(color_eyre::eyre::eyre!(
        "the desktop app currently targets macOS only"
    ))
}

#[cfg(test)]
mod tests {
    /// The packaging scripts and the binary must agree on the app identity.
    #[test]
    fn identity_matches_packaging_config() {
        let config = include_str!("../scripts/app-config.sh");
        let value = |key: &str| {
            config
                .lines()
                .find_map(|line| line.strip_prefix(&format!("{key}=")))
                .unwrap_or_else(|| panic!("{key} is missing from scripts/app-config.sh"))
                .to_owned()
        };
        assert_eq!(value("APP_NAME"), super::APP_NAME);
        assert_eq!(value("BUNDLE_ID"), super::BUNDLE_ID);
        assert_eq!(value("EXECUTABLE"), env!("CARGO_BIN_NAME"));
    }
}
