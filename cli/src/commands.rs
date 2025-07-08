use crate::config::Config;
use crate::utils::*;
use anyhow::Result;
use colored::*;
use std::path::Path;

pub async fn setup_command(verbose: bool) -> Result<()> {
    println!(
        "{} Setting up axonotes development environment...",
        "🚀".bright_blue()
    );

    // Load or create config
    let config = Config::load_or_create()?;

    // Check system dependencies
    if !check_rust_installed() {
        print_error("Rust is not installed");
        print_info("Install Rust from https://rustup.rs/");
        return Ok(());
    }

    if !check_bun_installed() {
        print_error("Bun is not installed");
        print_info("Install Bun from https://bun.sh/");
        return Ok(());
    }

    // Check and fix .env
    check_and_fix_env(&config).await?;

    // Check and install dashboard deps
    check_and_fix_dashboard_deps().await?;

    // Check and build CLI
    check_and_build_cli().await?;

    // Health report
    print_success("axonotes development environment setup complete!");
    println!("{} Next steps:", "🎉".bright_white());
    println!(
        "  {} - Start the development server",
        "axonotes server dev".bright_cyan()
    );
    println!(
        "  {} - Start the dashboard",
        "axonotes dashboard dev".bright_cyan()
    );

    Ok(())
}

pub async fn server_dev_command(
    persist: bool,
    auth: bool,
    verbose: bool,
) -> Result<()> {
    println!("{} Starting development server...", "🚀".bright_blue());

    let config = Config::load_or_create()?;

    // Smart checks
    check_and_build_cli().await?;
    check_and_fix_env(&config).await?;

    // Determine mode
    let mode = if persist { "persistent" } else { "memory" };
    let auth_status = if auth { "with auth" } else { "without auth" };

    println!(
        "{} Starting with {} database {}",
        "🔄".bright_green(),
        mode.bright_cyan(),
        auth_status.bright_yellow()
    );

    // Build args
    let oidc_issuer = format!("http://localhost:{}", config.dashboard.port);

    let mut args = vec![
        "start",
        "--allowed-oidc-issuer",
        &oidc_issuer,
        "--allowed-oidc-issuer",
        "https://auth.spacetimedb.com",
    ];

    if !persist {
        args.insert(1, "--in-memory");
    }

    if auth {
        args.push("--auth-required");
    }

    run_command_interactive("./bin/spacetimedb-cli", &args, None).await
}

pub async fn server_publish_command(verbose: bool) -> Result<()> {
    println!("{} Publishing server...", "📤".bright_blue());

    let config = Config::load_or_create()?;

    check_and_build_cli().await?;

    // Check if server compiles
    print_step("Checking server compilation...");
    if let Err(e) = run_command_quiet("cargo", &["check"], Some("server")).await
    {
        print_error("Server compilation failed");
        println!("{}", e);
        return Ok(());
    }

    // Publish
    run_command_interactive(
        "./bin/spacetimedb-cli",
        &[
            "publish",
            "--project-path",
            "server",
            "-c",
            &config.server.module_name,
        ],
        None,
    )
    .await?;

    // Generate bindings
    print_step("Generating TypeScript bindings...");
    std::fs::create_dir_all("dashboard/src/lib/module_bindings")?;
    run_command_quiet(
        "./bin/spacetimedb-cli",
        &[
            "generate",
            "--lang",
            "typescript",
            "--out-dir",
            "dashboard/src/lib/module_bindings",
            "--project-path",
            "server",
        ],
        None,
    )
    .await?;

    // Generate Tauri bindings
    print_step("Generating Tauri bindings...");
    std::fs::create_dir_all("app/src-tauri/src/module_bindings")?;
    run_command_quiet(
        "./bin/spacetimedb-cli",
        &[
            "generate",
            "--lang",
            "rust",
            "--out-dir",
            "app/src-tauri/src/module_bindings",
            "--project-path",
            "server",
        ],
        None,
    )
    .await?;

    print_success("Server published! Bindings updated in dashboard/");
    Ok(())
}

pub async fn dashboard_dev_command(verbose: bool) -> Result<()> {
    println!(
        "{} Starting dashboard development server...",
        "🚀".bright_blue()
    );

    let config = Config::load_or_create()?;

    check_and_fix_dashboard_deps().await?;
    check_and_fix_env(&config).await?;
    check_and_fix_bindings().await?;

    // Start with configured port
    let port_arg = format!("--port={}", config.dashboard.port);
    run_command_interactive(
        "bun",
        &["run", "dev", &port_arg],
        Some("dashboard"),
    )
    .await
}

pub async fn tauri_app_dev_command(verbose: bool) -> Result<()> {
    println!(
        "{} Starting Tauri app development server...",
        "🚀".bright_blue()
    );

    let config = Config::load_or_create()?;

    check_and_fix_tauri_app_deps().await?;
    check_and_fix_env(&config).await?;
    check_and_fix_bindings().await?;

    run_command_interactive("bun", &["run", "tauri", "dev"], Some("app")).await
}

pub async fn sdb_build_command(verbose: bool) -> Result<()> {
    build_spacetimedb_cli().await
}

pub async fn sdb_login_command(verbose: bool) -> Result<()> {
    check_and_build_cli().await?;
    run_command_interactive("./bin/spacetimedb-cli", &["login"], None).await
}

pub async fn sdb_logout_command(verbose: bool) -> Result<()> {
    check_and_build_cli().await?;
    run_command_interactive("./bin/spacetimedb-cli", &["logout"], None).await
}

pub async fn format_command(check: bool, verbose: bool) -> Result<()> {
    if check {
        println!("{} Checking code formatting...", "🔍".bright_blue());
        check_formatting().await
    } else {
        println!("{} Formatting code...", "🎨".bright_blue());
        format_code().await
    }
}

pub async fn dev_clean_command(verbose: bool) -> Result<()> {
    println!("{} Cleaning build artifacts...", "🧹".bright_blue());

    let items_to_clean = vec![
        "target/ directories",
        "node_modules/ directories",
        "bin/ directory",
        ".svelte-kit/ directory",
        "build/ directory",
    ];

    println!("This will delete:");
    for item in &items_to_clean {
        println!("  - {}", item);
    }

    if !confirm_action("Continue with cleanup?") {
        println!("Cleanup cancelled");
        return Ok(());
    }

    // Clean Rust targets
    let rust_dirs = ["server", "cli", "SpacetimeDB"];
    for dir in rust_dirs {
        if Path::new(&format!("{}/Cargo.toml", dir)).exists() {
            print_step(&format!("Cleaning {} artifacts...", dir));
            run_command_quiet("cargo", &["clean"], Some(dir)).await?;
        }
    }

    // Clean Node.js artifacts
    let node_dirs = ["dashboard", "scripts"];
    for dir in node_dirs {
        let node_modules = format!("{}/node_modules", dir);
        if Path::new(&node_modules).exists() {
            print_step(&format!("Removing {}/node_modules...", dir));
            std::fs::remove_dir_all(&node_modules)?;
        }
    }

    // Clean other artifacts
    if Path::new("bin").exists() {
        print_step("Removing bin directory...");
        std::fs::remove_dir_all("bin")?;
    }

    if Path::new("dashboard/.svelte-kit").exists() {
        print_step("Removing dashboard/.svelte-kit...");
        std::fs::remove_dir_all("dashboard/.svelte-kit")?;
    }

    if Path::new("dashboard/build").exists() {
        print_step("Removing dashboard/build...");
        std::fs::remove_dir_all("dashboard/build")?;
    }

    print_success("All build artifacts cleaned");
    Ok(())
}

// Helper functions
async fn check_and_build_cli() -> Result<()> {
    if !path_exists("bin/spacetimedb-cli") {
        print_warning("SpacetimeDB CLI not found");
        if confirm_action("Build SpacetimeDB CLI now?") {
            build_spacetimedb_cli().await?;
        } else {
            print_error("SpacetimeDB CLI is required");
            anyhow::bail!("CLI build cancelled");
        }
    }
    Ok(())
}

async fn build_spacetimedb_cli() -> Result<()> {
    print_step("Building SpacetimeDB CLI... (this may take a while)");

    if !path_exists("SpacetimeDB") {
        print_error("SpacetimeDB directory not found");
        print_info("Make sure you're in the project root directory");
        return Ok(());
    }

    std::fs::create_dir_all("bin")?;

    run_command_quiet(
        "cargo",
        &[
            "build",
            "--release",
            "-p",
            "spacetimedb-cli",
            "-p",
            "spacetimedb-standalone",
        ],
        Some("SpacetimeDB"),
    )
    .await?;

    // Copy binaries
    std::fs::copy(
        "SpacetimeDB/target/release/spacetimedb-cli",
        "bin/spacetimedb-cli",
    )?;
    std::fs::copy(
        "SpacetimeDB/target/release/spacetimedb-standalone",
        "bin/spacetimedb-standalone",
    )?;

    print_success("SpacetimeDB CLI built successfully");
    Ok(())
}

async fn check_and_fix_env(config: &Config) -> Result<()> {
    let env_path = "dashboard/.env";
    let example_env_path = "dashboard/.example.env";

    // Check if .example.env exists as reference
    if !path_exists(example_env_path) {
        print_warning(".example.env not found - cannot validate environment");
        return Ok(());
    }

    if !path_exists(env_path) {
        print_warning(".env file not found");
        if confirm_action("Generate .env file from .example.env?") {
            generate_env_file(config).await?;
        } else {
            print_error(".env file is required");
            anyhow::bail!("Environment setup cancelled");
        }
        return Ok(());
    }

    // Check if all required keys are populated
    let missing_keys = check_env_keys(env_path, example_env_path)?;

    if !missing_keys.is_empty() {
        print_warning(&format!(
            "Missing or empty environment variables: {}",
            missing_keys.join(", ")
        ));

        // Categorize missing keys - fix the type error
        let (jwt_keys, non_jwt_keys): (Vec<_>, Vec<_>) = missing_keys
            .into_iter()
            .partition(|key| key.starts_with("JWT_"));
        let (workos_keys, other_keys): (Vec<_>, Vec<_>) = non_jwt_keys
            .into_iter()
            .partition(|key| key.starts_with("WORKOS_"));

        // Handle JWT keys (we can auto-generate these)
        if !jwt_keys.is_empty() {
            print_info("JWT keys can be auto-generated");
            if confirm_action("Generate missing JWT keys?") {
                generate_jwt_keys().await?;
            }
        }

        // Handle WorkOS keys (user must configure these)
        if !workos_keys.is_empty() {
            print_error(
                "WorkOS credentials are missing and must be configured manually",
            );
            println!();
            println!("{}", "WorkOS Setup Instructions:".bright_yellow());
            println!(
                "1. Create free WorkOS account: https://workos.com/signin"
            );
            println!("2. Get your credentials from the dashboard:");
            println!("   - Navigate to 'API Keys' section");
            println!("   - Copy your Client ID and API Key (starts with sk_)");
            println!("3. Configure redirect URI in WorkOS dashboard:");
            println!("   - Go to 'Redirects' section");
            println!(
                "   - Add: http://localhost:{}/auth/callback",
                config.dashboard.port
            );
            println!(
                "   - Add: http://localhost:{}/auth/desktop/callback",
                config.dashboard.port
            );
            println!("4. Update your .env file with the credentials");
            println!();
            println!("Missing WorkOS variables: {}", workos_keys.join(", "));

            if !confirm_action(
                "Continue without WorkOS setup? (authentication will not work)",
            ) {
                anyhow::bail!("WorkOS setup required");
            }
        }

        // Handle other keys
        if !other_keys.is_empty() {
            print_warning(&format!(
                "Other missing variables: {}",
                other_keys.join(", ")
            ));
            print_info("Please check .example.env for expected values");
        }
    }

    // Check port consistency
    check_port_consistency(env_path, config).await?;

    Ok(())
}

fn check_env_keys(
    env_path: &str,
    example_env_path: &str,
) -> Result<Vec<String>> {
    let env_content = std::fs::read_to_string(env_path)?;
    let example_content = std::fs::read_to_string(example_env_path)?;

    // Extract required keys from .example.env (ignore comments and empty lines)
    let required_keys: Vec<String> = example_content
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
        .filter_map(|line| {
            if let Some(key) = line.split('=').next() {
                Some(key.trim().to_string())
            } else {
                None
            }
        })
        .collect();

    // Check which keys are missing or empty in actual .env
    let mut missing = Vec::new();

    for key in required_keys {
        if let Some(line) = env_content
            .lines()
            .find(|line| line.starts_with(&format!("{}=", key)))
        {
            // Check if value is empty - handle multiple empty formats
            let value_part = line.split('=').nth(1).unwrap_or("");
            let trimmed_value = value_part.trim();

            // Check for various empty formats: key=, key="", key='', key= (with spaces)
            if trimmed_value.is_empty()
                || trimmed_value == "\"\""
                || trimmed_value == "''"
                || (trimmed_value.starts_with('"')
                    && trimmed_value.ends_with('"')
                    && trimmed_value.len() == 2)
                || (trimmed_value.starts_with('\'')
                    && trimmed_value.ends_with('\'')
                    && trimmed_value.len() == 2)
            {
                missing.push(key);
            }
        } else {
            // Key doesn't exist at all
            missing.push(key);
        }
    }

    Ok(missing)
}

async fn generate_env_file(config: &Config) -> Result<()> {
    // Generate JWT keys
    print_step("Generating JWT keys...");
    generate_jwt_keys().await?;

    print_success("Environment file generated from .example.env");
    print_info("Please configure WorkOS credentials in dashboard/.env");

    Ok(())
}

async fn generate_jwt_keys() -> Result<()> {
    // Check if scripts are available
    if !path_exists("scripts") {
        print_error("Scripts directory not found");
        return Ok(());
    }

    // Install script dependencies if needed
    if !path_exists("scripts/node_modules") {
        print_step("Installing script dependencies...");
        run_command_quiet("bun", &["install"], Some("scripts")).await?;
    }

    // Generate keys
    run_command_interactive(
        "bun",
        &["run", "generate-keys.ts"],
        Some("scripts"),
    )
    .await?;

    print_success("JWT keys generated");
    Ok(())
}

async fn check_port_consistency(env_path: &str, config: &Config) -> Result<()> {
    let env_content = std::fs::read_to_string(env_path)?;
    let dashboard_port = config.dashboard.port;
    let mut needs_update = false;
    let mut mismatched_vars = Vec::new();

    // Check JWT_ISSUER
    if let Some(line) = env_content
        .lines()
        .find(|line| line.starts_with("JWT_ISSUER"))
    {
        if !line.contains(&format!("localhost:{}", dashboard_port)) {
            needs_update = true;
            mismatched_vars.push("JWT_ISSUER");
        }
    }

    // Check WORKOS_REDIRECT_URI
    if let Some(line) = env_content
        .lines()
        .find(|line| line.starts_with("WORKOS_REDIRECT_URI"))
    {
        if !line.contains(&format!("localhost:{}", dashboard_port)) {
            needs_update = true;
            mismatched_vars.push("WORKOS_REDIRECT_URI");
        }
    }

    // Check DESKTOP_AUTH_CALLBACK_URL
    if let Some(line) = env_content
        .lines()
        .find(|line| line.starts_with("DESKTOP_AUTH_CALLBACK_URL"))
    {
        if !line.contains(&format!("localhost:{}", dashboard_port)) {
            needs_update = true;
            mismatched_vars.push("DESKTOP_AUTH_CALLBACK_URL");
        }
    }

    if needs_update {
        print_warning(&format!(
            "Port mismatch detected in: {}",
            mismatched_vars.join(", ")
        ));
        println!("  Config port: {}", dashboard_port);
        println!("  Expected: http://localhost:{}", dashboard_port);

        if confirm_action("Update .env ports to match config?") {
            let updated_content = update_env_ports(&env_content, config);
            std::fs::write(env_path, updated_content)?;
            print_success("Environment file updated with correct ports");
        }
    }

    Ok(())
}

fn update_env_ports(env_content: &str, config: &Config) -> String {
    let dashboard_port = config.dashboard.port;

    env_content
        .lines()
        .map(|line| {
            if line.starts_with("JWT_ISSUER=") {
                format!("JWT_ISSUER=\"http://localhost:{}\"", dashboard_port)
            } else if line.starts_with("WORKOS_REDIRECT_URI=") {
                format!(
                    "WORKOS_REDIRECT_URI=\"http://localhost:{}/auth/callback\"",
                    dashboard_port
                )
            } else if line.starts_with("DESKTOP_AUTH_CALLBACK_URL=") {
                format!(
                    "DESKTOP_AUTH_CALLBACK_URL=\"http://localhost:{}/auth/desktop/callback\"",
                    dashboard_port
                )
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

async fn check_and_fix_dashboard_deps() -> Result<()> {
    if !path_exists("dashboard/node_modules") {
        print_warning("Dashboard dependencies not installed");
        if confirm_action("Install dashboard dependencies?") {
            print_step(
                "Installing dashboard dependencies... (this may take a while)",
            );
            run_command_quiet("bun", &["install"], Some("dashboard")).await?;
            print_success("Dashboard dependencies installed");
        } else {
            print_error("Dashboard dependencies are required");
            anyhow::bail!("Dependencies installation cancelled");
        }
    }
    Ok(())
}

async fn check_and_fix_tauri_app_deps() -> Result<()> {
    if !path_exists("app/node_modules") {
        print_warning("Tauri app dependencies not installed");
        if confirm_action("Install Tauri app dependencies?") {
            print_step(
                "Installing Tauri app dependencies... (this may take a while)",
            );
            run_command_quiet("bun", &["install"], Some("app")).await?;
            print_success("Tauri app dependencies installed");
        } else {
            print_error("Tauri app dependencies are required");
            anyhow::bail!("Dependencies installation cancelled");
        }
    }
    Ok(())
}

async fn check_and_fix_bindings() -> Result<()> {
    if !path_exists("dashboard/src/lib/module_bindings") {
        print_warning("TypeScript bindings missing");
        if confirm_action("Generate TypeScript bindings?") {
            check_and_build_cli().await?;
            print_step("Generating TypeScript bindings...");
            std::fs::create_dir_all("dashboard/src/lib/module_bindings")?;
            run_command_quiet(
                "./bin/spacetimedb-cli",
                &[
                    "generate",
                    "--lang",
                    "typescript",
                    "--out-dir",
                    "dashboard/src/lib/module_bindings",
                    "--project-path",
                    "server",
                ],
                None,
            )
            .await?;
            print_success("TypeScript bindings generated");
        }
    }
    if !path_exists("app/src-tauri/src/module_bindings") {
        print_warning("Tauri bindings missing");
        if confirm_action("Generate Tauri bindings?") {
            check_and_build_cli().await?;
            print_step("Generating Tauri bindings...");
            std::fs::create_dir_all("app/src-tauri/src/module_bindings")?;
            run_command_quiet(
                "./bin/spacetimedb-cli",
                &[
                    "generate",
                    "--lang",
                    "rust",
                    "--out-dir",
                    "app/src-tauri/src/module_bindings",
                    "--project-path",
                    "server",
                ],
                None,
            )
            .await?;
            print_success("Tauri bindings generated");
        }
    }
    Ok(())
}

async fn format_code() -> Result<()> {
    // Format TypeScript/JavaScript
    if path_exists("package.json") {
        print_step("Formatting TypeScript/JavaScript files...");
        run_command_quiet("prettier", &["--write", "."], None).await?;
    }

    // Format Rust code
    let rust_dirs = ["server", "cli", "SpacetimeDB"];
    for dir in rust_dirs {
        if Path::new(&format!("{}/Cargo.toml", dir)).exists() {
            print_step(&format!("Formatting {} Rust code...", dir));
            run_command_quiet("cargo", &["fmt", "--all"], Some(dir)).await?;
        }
    }

    print_success("All code formatted");
    Ok(())
}

async fn check_formatting() -> Result<()> {
    let mut all_formatted = true;

    // Check TypeScript/JavaScript
    if path_exists("package.json") {
        print_step("Checking TypeScript/JavaScript formatting...");
        if run_command_quiet("prettier", &["--check", "."], None)
            .await
            .is_err()
        {
            print_error("TypeScript/JavaScript files need formatting");
            all_formatted = false;
        }
    }

    // Check Rust code
    let rust_dirs = ["server", "cli", "SpacetimeDB"];
    for dir in rust_dirs {
        if Path::new(&format!("{}/Cargo.toml", dir)).exists() {
            print_step(&format!("Checking {} Rust formatting...", dir));
            if run_command_quiet(
                "cargo",
                &["fmt", "--all", "--check"],
                Some(dir),
            )
            .await
            .is_err()
            {
                print_error(&format!("{} Rust code needs formatting", dir));
                all_formatted = false;
            }
        }
    }

    if all_formatted {
        print_success("All code is properly formatted");
    } else {
        print_error(
            "Some files need formatting. Run 'axonotes format' to fix.",
        );
        anyhow::bail!("Code formatting check failed");
    }

    Ok(())
}
