use serde::Deserialize;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("No .aegis.yaml found walking up from {}", .0.display())]
    NotFound(PathBuf),
}

#[derive(Debug, Deserialize, Clone)]
pub struct AegisConfig {
    #[allow(dead_code)]
    pub version: u8,
    #[serde(default)]
    pub catalog: Option<String>,
    pub rules: Vec<ScanRule>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ScanRule {
    pub id: String,
    pub files: Vec<String>,
    pub patterns: Vec<PatternDef>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PatternDef {
    pub regex: String,
    pub level: String,
    #[serde(default)]
    pub capture_mode: CaptureMode,
    #[serde(default)]
    pub sub_pattern: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CaptureMode {
    #[default]
    Single,
    Repeated,
}

pub fn discover_config(start_dir: &Path) -> Result<(PathBuf, AegisConfig), ConfigError> {
    let mut current = Some(start_dir.to_path_buf());

    while let Some(dir) = current {
        let candidate = dir.join(".aegis.yaml");
        if candidate.exists() {
            let content = std::fs::read_to_string(&candidate)?;
            let config: AegisConfig = serde_yaml::from_str(&content)?;
            return Ok((candidate, config));
        }
        current = dir.parent().map(|p| p.to_path_buf());
    }

    Err(ConfigError::NotFound(start_dir.to_path_buf()))
}

pub fn parse_config(path: &Path) -> Result<AegisConfig, ConfigError> {
    let content = std::fs::read_to_string(path)?;
    Ok(serde_yaml::from_str(&content)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_config() {
        let yaml = r#"
version: 1
rules:
  - id: test-rule
    files:
      - "**/*.tsx"
    patterns:
      - regex: '<Can\s+object="(?P<object>[^"]+)"\s+action="(?P<action>[^"]+)"'
        level: ui
"#;
        let config: AegisConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.rules.len(), 1);
        assert_eq!(config.rules[0].id, "test-rule");
        assert_eq!(config.rules[0].patterns[0].level, "ui");
        assert_eq!(
            config.rules[0].patterns[0].capture_mode,
            CaptureMode::Single
        );
    }

    #[test]
    fn test_parse_catalog_alias() {
        let yaml = r#"
version: 1
catalog: config/permissions.json
rules:
  - id: test-rule
    files: ["**/*.go"]
    patterns:
      - regex: 'x'
        level: api
"#;
        let config: AegisConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.catalog.as_deref(), Some("config/permissions.json"));
    }

    #[test]
    fn test_parse_catalog_defaults_none() {
        let yaml = r#"
version: 1
rules:
  - id: test-rule
    files: ["**/*.go"]
    patterns:
      - regex: 'x'
        level: api
"#;
        let config: AegisConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.catalog.is_none());
    }

    #[test]
    fn test_parse_repeated_mode() {
        let yaml = r#"
version: 1
rules:
  - id: multi
    files: ["**/*.tsx"]
    patterns:
      - regex: 'all=\{'
        level: ui
        capture_mode: repeated
        sub_pattern: 'object:\s*"(?P<object>[^"]+)"'
"#;
        let config: AegisConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            config.rules[0].patterns[0].capture_mode,
            CaptureMode::Repeated
        );
        assert!(config.rules[0].patterns[0].sub_pattern.is_some());
    }
}
