use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;

mod commands;
mod config;
mod utils;

#[derive(Parser)]
#[command(
    name = "axonotes",
    version,
    about = "axonotes Development CLI",
    long_about = "A smart CLI tool for axonotes development"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Setup development environment
    Setup,

    /// Server operations
    #[command(subcommand)]
    Server(ServerCommands),

    /// Dashboard operations  
    #[command(subcommand)]
    Dashboard(DashboardCommands),

    /// Tauri app operations
    #[command(subcommand)]
    App(TauriAppCommands),

    /// SpacetimeDB operations
    #[command(subcommand)]
    Sdb(SdbCommands),

    /// Development utilities
    #[command(subcommand)]
    Dev(DevCommands),

    /// Format code
    Format {
        /// Check formatting without applying changes
        #[arg(long)]
        check: bool,
    },
}

#[derive(Subcommand)]
enum ServerCommands {
    /// Start development server
    Dev {
        /// Use persistent database
        #[arg(long)]
        persist: bool,
        /// Enable authentication
        #[arg(long)]
        auth: bool,
    },
    /// Publish server and generate bindings
    Publish,
}

#[derive(Subcommand)]
enum DashboardCommands {
    /// Start development server
    Dev,
}

#[derive(Subcommand)]
enum TauriAppCommands {
    /// Run the Tauri app in development mode
    Dev,
}

#[derive(Subcommand)]
enum SdbCommands {
    /// Build SpacetimeDB CLI
    Build,
    /// Login to SpacetimeDB
    Login,
    /// Logout from SpacetimeDB
    Logout,
}

#[derive(Subcommand)]
enum DevCommands {
    /// Clean build artifacts
    Clean,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Find and set project root
    let project_root = find_project_root()?;
    std::env::set_current_dir(&project_root)?;

    if cli.verbose {
        println!(
            "{} {}",
            "🏠 Project root:".bright_black(),
            project_root.display()
        );
    }

    // Dispatch commands
    let result = match cli.command {
        Commands::Setup => commands::setup_command(cli.verbose).await,
        Commands::Server(ServerCommands::Dev { persist, auth }) => {
            commands::server_dev_command(persist, auth, cli.verbose).await
        }
        Commands::Server(ServerCommands::Publish) => {
            commands::server_publish_command(cli.verbose).await
        }
        Commands::Dashboard(DashboardCommands::Dev) => {
            commands::dashboard_dev_command(cli.verbose).await
        }
        Commands::App(TauriAppCommands::Dev) => {
            commands::tauri_app_dev_command(cli.verbose).await
        }
        Commands::Sdb(SdbCommands::Build) => {
            commands::sdb_build_command(cli.verbose).await
        }
        Commands::Sdb(SdbCommands::Login) => {
            commands::sdb_login_command(cli.verbose).await
        }
        Commands::Sdb(SdbCommands::Logout) => {
            commands::sdb_logout_command(cli.verbose).await
        }
        Commands::Dev(DevCommands::Clean) => {
            commands::dev_clean_command(cli.verbose).await
        }
        Commands::Format { check } => {
            commands::format_command(check, cli.verbose).await
        }
    };

    if let Err(e) = result {
        utils::print_error(&e.to_string());
        std::process::exit(1);
    }

    Ok(())
}

fn find_project_root() -> Result<PathBuf> {
    let current_dir = std::env::current_dir()?;

    // If we're in the cli directory, go up one level
    if current_dir.file_name() == Some(std::ffi::OsStr::new("cli")) {
        if let Some(parent) = current_dir.parent() {
            return Ok(parent.to_path_buf());
        }
    }

    // Look for project root markers
    let mut dir = current_dir.as_path();
    loop {
        if dir.join("package.json").exists()
            && dir.join("dashboard").exists()
            && dir.join("server").exists()
            && dir.join("SpacetimeDB").exists()
        {
            return Ok(dir.to_path_buf());
        }

        if let Some(parent) = dir.parent() {
            dir = parent;
        } else {
            break;
        }
    }

    anyhow::bail!(
        "Could not find axonotes project root. Make sure you're in the project directory."
    );
}
