use anyhow::Result;
use colored::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub dashboard: DashboardConfig,
    pub spacetimedb: SpacetimeDbConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    pub default_mode: String,
    pub auth_required: bool,
    pub module_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DashboardConfig {
    pub port: u16,
    pub auto_install: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SpacetimeDbConfig {
    pub url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                default_mode: "memory".to_string(),
                auth_required: false,
                module_name: "axonotes".to_string(),
            },
            dashboard: DashboardConfig {
                port: 5173,
                auto_install: true,
            },
            spacetimedb: SpacetimeDbConfig {
                url: "https://testnet.spacetimedb.com".to_string(),
            },
        }
    }
}

impl Config {
    pub fn load_or_create() -> Result<Self> {
        let config_path = Path::new("axonotes.toml");

        if config_path.exists() {
            let content = std::fs::read_to_string(config_path)?;
            match toml::from_str(&content) {
                Ok(config) => Ok(config),
                Err(e) => {
                    println!(
                        "{} Invalid axonotes.toml format: {}",
                        "⚠️".bright_yellow(),
                        e
                    );
                    println!("{} Using default config", "🔧".bright_cyan());
                    Ok(Config::default())
                }
            }
        } else {
            println!(
                "{} axonotes.toml not found, creating with defaults",
                "🔧".bright_cyan()
            );
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write("axonotes.toml", content)?;
        Ok(())
    }
}
