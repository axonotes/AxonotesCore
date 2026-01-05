use crate::config::Config;
use crate::utils::{AppError, Result};
use rhai::{Dynamic, Engine, Scope};
use serde_json::Value;
use tracing::{debug, warn};

/// Quota rule (stores condition as string for Send + Sync)
#[derive(Clone)]
struct QuotaRuleConfig {
    name: String,
    condition: String,
    quota_bytes: u64,
}

/// Quota rule evaluator (Send + Sync safe)
pub struct QuotaEvaluator {
    rules: Vec<QuotaRuleConfig>,
    default_quota_bytes: u64,
}

impl QuotaEvaluator {
    /// Create evaluator from config
    pub fn from_config(config: &Config) -> Result<Self> {
        let mut rules = Vec::new();

        // Store rules as strings (not compiled AST) for Send + Sync
        for rule in &config.quota_rules.rule {
            // Validate condition compiles
            let engine = Engine::new();
            engine.compile(&rule.condition).map_err(|e| {
                AppError::Config(format!("Failed to compile rule '{}': {}", rule.name, e))
            })?;

            rules.push(QuotaRuleConfig {
                name: rule.name.clone(),
                condition: rule.condition.clone(),
                quota_bytes: Config::quota_gb_to_bytes(rule.quota_gb),
            });
        }

        let default_quota_bytes = Config::quota_gb_to_bytes(config.quota_rules.default.quota_gb);

        Ok(Self {
            rules,
            default_quota_bytes,
        })
    }

    /// Evaluate rules against JWT claims and return quota in bytes + matched rule name
    pub fn evaluate(&self, claims: &Value) -> Result<(u64, String)> {
        // Create engine for evaluation
        let engine = Engine::new();
        let mut scope = Scope::new();

        // Add JWT claims to scope
        Self::add_claims_to_scope(&mut scope, "jwt", claims);

        // Evaluate rules in order, return highest matching quota
        let mut max_quota = 0u64;
        let mut matched_rule = "default".to_string();

        for rule in &self.rules {
            match engine.eval_with_scope::<bool>(&mut scope, &rule.condition) {
                Ok(true) => {
                    debug!("Rule '{}' matched", rule.name);
                    if rule.quota_bytes > max_quota {
                        max_quota = rule.quota_bytes;
                        matched_rule = rule.name.clone();
                    }
                }
                Ok(false) => {
                    debug!("Rule '{}' did not match", rule.name);
                }
                Err(e) => {
                    // Log error at warn level - this indicates a misconfigured rule
                    warn!(
                        "Quota rule '{}' evaluation failed (check condition syntax): {}",
                        rule.name, e
                    );
                }
            }
        }

        // Use default if no rules matched
        if max_quota == 0 {
            max_quota = self.default_quota_bytes;
            matched_rule = "default".to_string();
        }

        Ok((max_quota, matched_rule))
    }

    /// Recursively add JSON value to Rhai scope
    fn add_claims_to_scope(scope: &mut Scope, prefix: &str, value: &Value) {
        match value {
            Value::Object(map) => {
                // Create a Rhai map for the object
                let mut rhai_map = rhai::Map::new();

                for (key, val) in map {
                    let dynamic_val = Self::json_to_dynamic(val);
                    rhai_map.insert(key.clone().into(), dynamic_val);
                }

                scope.push(prefix, rhai_map);
            }
            _ => {
                // For non-objects, just convert directly
                scope.push(prefix, Self::json_to_dynamic(value));
            }
        }
    }

    /// Convert JSON value to Rhai Dynamic
    fn json_to_dynamic(value: &Value) -> Dynamic {
        match value {
            Value::Null => Dynamic::UNIT,
            Value::Bool(b) => Dynamic::from(*b),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Dynamic::from(i)
                } else if let Some(f) = n.as_f64() {
                    Dynamic::from(f)
                } else {
                    Dynamic::UNIT
                }
            }
            Value::String(s) => Dynamic::from(s.clone()),
            Value::Array(arr) => {
                let rhai_arr: Vec<Dynamic> = arr.iter().map(Self::json_to_dynamic).collect();
                Dynamic::from(rhai_arr)
            }
            Value::Object(map) => {
                let mut rhai_map = rhai::Map::new();
                for (key, val) in map {
                    rhai_map.insert(key.clone().into(), Self::json_to_dynamic(val));
                }
                Dynamic::from(rhai_map)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::QuotaRule;
    use serde_json::json;

    fn create_test_config(rules: Vec<QuotaRule>, default_gb: u64) -> Config {
        Config {
            jwt: crate::config::JwtConfig {
                issuers: vec![],
                dev_mode: true,
                signature_max_age_secs: 300,
            },
            storage: crate::config::StorageConfig {
                endpoint: "http://localhost:9000".to_string(),
                bucket: "test".to_string(),
                access_key: "test".to_string(),
                secret_key: "test".to_string(),
                region: None,
                max_blob_size_mb: 100,
            },
            database: crate::config::DatabaseConfig::default(),
            logging: crate::config::LoggingConfig::default(),
            rate_limits: crate::config::RateLimitConfig::default(),
            quota_rules: crate::config::QuotaRulesConfig {
                rule: rules,
                default: crate::config::DefaultQuota {
                    quota_gb: default_gb,
                },
            },
        }
    }

    #[test]
    fn test_default_quota() {
        let config = create_test_config(vec![], 1);
        let evaluator = QuotaEvaluator::from_config(&config).unwrap();

        let claims = json!({});
        let (quota, rule) = evaluator.evaluate(&claims).unwrap();

        assert_eq!(quota, 1_073_741_824); // 1 GB
        assert_eq!(rule, "default");
    }

    #[test]
    fn test_simple_rule() {
        let rules = vec![QuotaRule {
            name: "pro_users".to_string(),
            condition: r#"jwt.plan == "pro""#.to_string(),
            quota_gb: 50,
        }];

        let config = create_test_config(rules, 1);
        let evaluator = QuotaEvaluator::from_config(&config).unwrap();

        let claims = json!({ "plan": "pro" });
        let (quota, rule) = evaluator.evaluate(&claims).unwrap();

        assert_eq!(quota, 53_687_091_200); // 50 GB
        assert_eq!(rule, "pro_users");
    }

    #[test]
    fn test_multiple_rules_highest_wins() {
        let rules = vec![
            QuotaRule {
                name: "pro_users".to_string(),
                condition: r#"jwt.plan == "pro""#.to_string(),
                quota_gb: 50,
            },
            QuotaRule {
                name: "enterprise_users".to_string(),
                condition: r#"jwt.contains("org_id")"#.to_string(),
                quota_gb: 500,
            },
        ];

        let config = create_test_config(rules, 1);
        let evaluator = QuotaEvaluator::from_config(&config).unwrap();

        let claims = json!({ "plan": "pro", "org_id": "org_123" });
        let (quota, rule) = evaluator.evaluate(&claims).unwrap();

        assert_eq!(quota, 536_870_912_000); // 500 GB (highest)
        assert_eq!(rule, "enterprise_users");
    }

    #[test]
    fn test_complex_condition() {
        let rules = vec![QuotaRule {
            name: "special_users".to_string(),
            condition: r#"(jwt.plan == "pro" || jwt.trial == true) && jwt.verified == true"#
                .to_string(),
            quota_gb: 100,
        }];

        let config = create_test_config(rules, 1);
        let evaluator = QuotaEvaluator::from_config(&config).unwrap();

        // Should match
        let claims = json!({ "plan": "pro", "verified": true });
        let (quota, _) = evaluator.evaluate(&claims).unwrap();
        assert_eq!(quota, 107_374_182_400); // 100 GB

        // Should not match (not verified)
        let claims = json!({ "plan": "pro", "verified": false });
        let (quota, rule) = evaluator.evaluate(&claims).unwrap();
        assert_eq!(quota, 1_073_741_824); // Default 1 GB
        assert_eq!(rule, "default");
    }

    #[test]
    fn test_null_check() {
        let rules = vec![QuotaRule {
            name: "has_org".to_string(),
            condition: r#"jwt.contains("org_id")"#.to_string(),
            quota_gb: 200,
        }];

        let config = create_test_config(rules, 1);
        let evaluator = QuotaEvaluator::from_config(&config).unwrap();

        // With org_id
        let claims = json!({ "org_id": "org_123" });
        let (quota, rule) = evaluator.evaluate(&claims).unwrap();
        assert_eq!(quota, 214_748_364_800); // 200 GB
        assert_eq!(rule, "has_org");

        // Without org_id
        let claims = json!({});
        let (quota, rule) = evaluator.evaluate(&claims).unwrap();
        assert_eq!(quota, 1_073_741_824); // Default
        assert_eq!(rule, "default");
    }
}
