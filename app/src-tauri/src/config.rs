use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub dashboard_url: String,
    pub spacetimedb_url: String,
    pub spacetimedb_database: String,
    pub log_level: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            dashboard_url: "http://localhost:5173".to_string(),
            spacetimedb_url: "ws://localhost:3000".to_string(),
            spacetimedb_database: "axonotes".to_string(),
            log_level: "info".to_string(),
        }
    }
}

impl AppConfig {
    pub fn config_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let project_dirs = ProjectDirs::from("com", "axonotes", "Axonotes")
            .ok_or("Failed to determine project directories")?;

        let config_dir = project_dirs.config_dir();
        std::fs::create_dir_all(config_dir)?;
        Ok(config_dir.to_path_buf())
    }

    pub fn config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let config_str = std::fs::read_to_string(&config_path)?;
            let config: AppConfig = toml::from_str(&config_str)?;
            Ok(config)
        } else {
            // Create default config
            let config = Self::default();
            config.save()?;
            log::info!("Created default config at: {}", config_path.display());
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::config_path()?;
        let config_str = toml::to_string_pretty(self)?;
        std::fs::write(&config_path, config_str)?;
        log::info!("Saved config to: {}", config_path.display());
        Ok(())
    }

    pub fn get_config_path_string() -> String {
        Self::config_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "Unable to determine config path".to_string())
    }
}
