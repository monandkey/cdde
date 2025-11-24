use serde::{Deserialize, Serialize};
use thiserror::Error;
use validator::Validate;

/// Configuration error
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to load config: {0}")]
    LoadError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),
}

/// Common application configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AppConfig {
    #[validate(length(min = 1))]
    pub service_name: String,
    #[validate(length(min = 1))]
    pub log_level: String,
    #[validate(range(min = 1, max = 65535))]
    pub metrics_port: u16,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            service_name: "cdde".to_string(),
            log_level: "info".to_string(),
            metrics_port: 9090,
        }
    }
}

/// Load configuration from file
pub fn load_config<T>(path: &str) -> Result<T, ConfigError>
where
    T: for<'de> Deserialize<'de> + Validate,
{
    let config: T = config::Config::builder()
        .add_source(config::File::with_name(path))
        .add_source(config::Environment::with_prefix("CDDE"))
        .build()
        .map_err(|e| ConfigError::LoadError(e.to_string()))?
        .try_deserialize()
        .map_err(|e| ConfigError::LoadError(e.to_string()))?;

    config
        .validate()
        .map_err(|e| ConfigError::ValidationError(e.to_string()))?;
    Ok(config)
}

/// Load configuration from YAML string (for testing)
pub fn load_from_yaml<T>(yaml: &str) -> Result<T, ConfigError>
where
    T: for<'de> Deserialize<'de> + Validate,
{
    let config: T =
        serde_yaml::from_str(yaml).map_err(|e| ConfigError::LoadError(e.to_string()))?;
    config
        .validate()
        .map_err(|e| ConfigError::ValidationError(e.to_string()))?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.service_name, "cdde");
        assert_eq!(config.log_level, "info");
        assert_eq!(config.metrics_port, 9090);
    }

    #[test]
    fn test_load_from_yaml() {
        let yaml = r#"
service_name: test-service
log_level: debug
metrics_port: 8080
"#;
        let config: AppConfig = load_from_yaml(yaml).unwrap();
        assert_eq!(config.service_name, "test-service");
        assert_eq!(config.log_level, "debug");
        assert_eq!(config.metrics_port, 8080);
    }

    #[test]
    fn test_validation_error() {
        let yaml = r#"
service_name: ""
log_level: info
metrics_port: 9090
"#;
        let result: Result<AppConfig, _> = load_from_yaml(yaml);
        assert!(result.is_err());
        match result {
            Err(ConfigError::ValidationError(_)) => (), // Expected
            _ => panic!("Expected ValidationError"),
        }
    }
}
