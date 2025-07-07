use anyhow::Result;
use colored::*;
use dialoguer::Confirm;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

pub async fn run_command_interactive(
    cmd: &str,
    args: &[&str],
    working_dir: Option<&str>,
) -> Result<()> {
    let mut command = Command::new(cmd);
    command.args(args);

    if let Some(dir) = working_dir {
        command.current_dir(dir);
    }

    command.stdin(Stdio::inherit());
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());

    let status = command.status().await?;

    if !status.success() {
        anyhow::bail!("Command failed: {} {}", cmd, args.join(" "));
    }

    Ok(())
}

pub async fn run_command_quiet(
    cmd: &str,
    args: &[&str],
    working_dir: Option<&str>,
) -> Result<String> {
    let mut command = Command::new(cmd);
    command.args(args);

    if let Some(dir) = working_dir {
        command.current_dir(dir);
    }

    let output = command.output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "Command failed: {} {}\nError: {}",
            cmd,
            args.join(" "),
            stderr
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn path_exists(path: &str) -> bool {
    Path::new(path).exists()
}

pub fn confirm_action(message: &str) -> bool {
    Confirm::new()
        .with_prompt(message)
        .default(false)
        .interact()
        .unwrap_or(false)
}

pub fn print_error(msg: &str) {
    println!("{} {}", "❌".bright_red(), msg);
}

pub fn print_warning(msg: &str) {
    println!("{} {}", "⚠️".bright_yellow(), msg);
}

pub fn print_info(msg: &str) {
    println!("{} {}", "ℹ️".bright_blue(), msg);
}

pub fn print_success(msg: &str) {
    println!("{} {}", "✅".bright_green(), msg);
}

pub fn print_step(msg: &str) {
    println!("{} {}", "🔧".bright_cyan(), msg);
}

pub fn check_rust_installed() -> bool {
    std::process::Command::new("cargo")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub fn check_bun_installed() -> bool {
    std::process::Command::new("bun")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
