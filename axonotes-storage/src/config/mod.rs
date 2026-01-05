pub mod hot_reload;

use crate::utils::{AppError, Result};
use serde::Deserialize;
use std::path::Path;

/// Main configuration structure
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub jwt: JwtConfig,
    pub storage: StorageConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub rate_limits: RateLimitConfig,
    pub quota_rules: QuotaRulesConfig,
}

/// JWT configuration
#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    /// List of trusted JWT issuers
    #[serde(default)]
    pub issuers: Vec<String>,
    /// Development mode - skips signature verification (NEVER use in production!)
    #[serde(default)]
    pub dev_mode: bool,
    /// Maximum signature timestamp age in seconds (default: 300 = 5 minutes)
    #[serde(default = "default_signature_max_age")]
    pub signature_max_age_secs: u64,
}

fn default_signature_max_age() -> u64 {
    300 // 5 minutes
}

/// S3 storage configuration
#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    pub endpoint: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
    pub region: Option<String>,
    /// Maximum blob size in MB (default: 100 MB)
    #[serde(default = "default_max_blob_size_mb")]
    pub max_blob_size_mb: u64,
}

fn default_max_blob_size_mb() -> u64 {
    100 // 100 MB default
}

/// Database configuration
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_db_path")]
    pub path: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: default_db_path(),
        }
    }
}

fn default_db_path() -> String {
    "./data/metadata.db".to_string()
}

/// Logging configuration
#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_file")]
    pub file: String,
    #[serde(default = "default_log_max_size_mb")]
    #[allow(dead_code)]
    pub max_size_mb: u64,
    #[serde(default = "default_log_max_age_days")]
    pub max_age_days: u64,
    #[serde(default = "default_log_compress")]
    #[allow(dead_code)]
    pub compress: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            file: default_log_file(),
            max_size_mb: default_log_max_size_mb(),
            max_age_days: default_log_max_age_days(),
            compress: default_log_compress(),
        }
    }
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_file() -> String {
    "./logs/app.log".to_string()
}

fn default_log_max_size_mb() -> u64 {
    100
}

fn default_log_max_age_days() -> u64 {
    30
}

fn default_log_compress() -> bool {
    true
}

/// Rate limiting configuration
#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    /// Upload rate limit: requests per window
    #[serde(default = "default_upload_limit")]
    pub upload_per_minute: u32,
    /// Download rate limit: requests per window
    #[serde(default = "default_download_limit")]
    pub download_per_minute: u32,
    /// Quota check rate limit: requests per window
    #[serde(default = "default_quota_limit")]
    pub quota_per_minute: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            upload_per_minute: default_upload_limit(),
            download_per_minute: default_download_limit(),
            quota_per_minute: default_quota_limit(),
        }
    }
}

fn default_upload_limit() -> u32 {
    10 // 10 uploads per minute
}

fn default_download_limit() -> u32 {
    100 // 100 downloads per minute
}

fn default_quota_limit() -> u32 {
    60 // 60 quota checks per minute
}

/// Quota rules configuration
#[derive(Debug, Clone, Deserialize)]
pub struct QuotaRulesConfig {
    #[serde(default)]
    pub rule: Vec<QuotaRule>,
    pub default: DefaultQuota,
}

/// Individual quota rule
#[derive(Debug, Clone, Deserialize)]
pub struct QuotaRule {
    pub name: String,
    pub condition: String,
    pub quota_gb: u64,
}

/// Default quota (required)
#[derive(Debug, Clone, Deserialize)]
pub struct DefaultQuota {
    pub quota_gb: u64,
}

impl Config {
    /// Load configuration from TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| AppError::Config(format!("Failed to read config file: {}", e)))?;

        let config: Config = toml::from_str(&content)
            .map_err(|e| AppError::Config(format!("Failed to parse config: {}", e)))?;

        config.validate()?;
        Ok(config)
    }

    /// Validate configuration
    fn validate(&self) -> Result<()> {
        // Validate JWT config
        if !self.jwt.dev_mode && self.jwt.issuers.is_empty() {
            return Err(AppError::Config(
                "jwt.issuers must not be empty when dev_mode is false. \
                 Set jwt.dev_mode = true for development or add trusted issuers."
                    .to_string(),
            ));
        }

        // Validate storage config
        if self.storage.endpoint.is_empty() {
            return Err(AppError::Config("storage.endpoint is required".to_string()));
        }
        if self.storage.bucket.is_empty() {
            return Err(AppError::Config("storage.bucket is required".to_string()));
        }
        if self.storage.access_key.is_empty() {
            return Err(AppError::Config(
                "storage.access_key is required".to_string(),
            ));
        }
        if self.storage.secret_key.is_empty() {
            return Err(AppError::Config(
                "storage.secret_key is required".to_string(),
            ));
        }
        if self.storage.max_blob_size_mb == 0 {
            return Err(AppError::Config(
                "storage.max_blob_size_mb must be greater than 0".to_string(),
            ));
        }

        // Validate default quota exists
        if self.quota_rules.default.quota_gb == 0 {
            return Err(AppError::Config(
                "quota_rules.default.quota_gb must be greater than 0".to_string(),
            ));
        }

        // Validate quota rules have non-empty conditions
        for rule in &self.quota_rules.rule {
            if rule.condition.is_empty() {
                return Err(AppError::Config(format!(
                    "Quota rule '{}' has empty condition",
                    rule.name
                )));
            }
            if rule.quota_gb == 0 {
                return Err(AppError::Config(format!(
                    "Quota rule '{}' has zero quota_gb",
                    rule.name
                )));
            }
        }

        Ok(())
    }

    /// Convert GB to bytes
    pub fn quota_gb_to_bytes(gb: u64) -> u64 {
        gb * 1024 * 1024 * 1024
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quota_gb_to_bytes() {
        assert_eq!(Config::quota_gb_to_bytes(1), 1_073_741_824);
        assert_eq!(Config::quota_gb_to_bytes(50), 53_687_091_200);
    }

    #[test]
    fn test_default_config_values() {
        let db_config = DatabaseConfig::default();
        assert_eq!(db_config.path, "./data/metadata.db");

        let log_config = LoggingConfig::default();
        assert_eq!(log_config.level, "info");
        assert_eq!(log_config.max_size_mb, 100);
        assert_eq!(log_config.compress, true);
    }
}
