use glob::Pattern;
use rayon::prelude::*;
use std::io::Read;
use std::path::Path;
use walkdir::WalkDir;

use crate::config::AegisConfig;
use crate::pattern::{CompiledPattern, ScanResult};

pub fn scan(config: &AegisConfig, root: &Path, ignore_rules: &[String]) -> Vec<ScanResult> {
    let compiled: Vec<(Vec<String>, Vec<CompiledPattern>)> = config
        .rules
        .iter()
        .filter(|rule| !ignore_rules.contains(&rule.id))
        .filter_map(|rule| {
            let compiled_patterns: Vec<CompiledPattern> = rule
                .patterns
                .iter()
                .filter_map(|p| CompiledPattern::compile(&rule.id, p).ok())
                .collect();
            if compiled_patterns.is_empty() {
                None
            } else {
                Some((rule.files.clone(), compiled_patterns))
            }
        })
        .collect();

    let entries: Vec<_> = WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .collect();

    entries
        .par_iter()
        .flat_map(|entry| {
            let path = entry.path();
            let relative = path.strip_prefix(root).unwrap_or(path);
            let relative_str = relative.to_string_lossy();

            let applicable: Vec<&CompiledPattern> = compiled
                .iter()
                .filter(|(globs, _)| {
                    globs.iter().any(|g| {
                        Pattern::new(g)
                            .map(|p| p.matches(&relative_str))
                            .unwrap_or(false)
                    })
                })
                .flat_map(|(_, cps)| cps.iter())
                .collect();

            if applicable.is_empty() {
                return Vec::new();
            }

            let mut file = match std::fs::File::open(path) {
                Ok(f) => f,
                Err(_) => return Vec::new(),
            };
            let mut content = String::new();
            if file.read_to_string(&mut content).is_err() {
                return Vec::new();
            }

            applicable
                .iter()
                .flat_map(|cp| crate::pattern::extract_matches(&content, 0, cp, &relative_str))
                .collect::<Vec<_>>()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AegisConfig, CaptureMode, PatternDef, ScanRule};
    use std::io::Write;

    fn make_temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    fn write_file(dir: &Path, name: &str, content: &str) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    fn test_config() -> AegisConfig {
        AegisConfig {
            version: 1,
            rules: vec![
                ScanRule {
                    id: "react-can".to_string(),
                    files: vec!["**/*.tsx".to_string()],
                    patterns: vec![PatternDef {
                        regex: r#"<Can\s+object="(?P<object>[^"]+)"\s+action="(?P<action>[^"]+)"#
                            .to_string(),
                        level: "ui".to_string(),
                        capture_mode: CaptureMode::Single,
                        sub_pattern: None,
                    }],
                },
                ScanRule {
                    id: "go-check".to_string(),
                    files: vec!["**/*.go".to_string()],
                    patterns: vec![PatternDef {
                        regex: r#"CheckAny\(.*?,\s*"(?P<object>[^"]+)",\s*"(?P<action>[^"]+)"\)"#
                            .to_string(),
                        level: "api".to_string(),
                        capture_mode: CaptureMode::Single,
                        sub_pattern: None,
                    }],
                },
            ],
        }
    }

    #[test]
    fn test_scanner_finds_react_permissions() {
        let dir = make_temp_dir();
        write_file(
            dir.path(),
            "src/App.tsx",
            r#"<Can object="api:packages" action="delete"><button/></Can>"#,
        );

        let results = scan(&test_config(), dir.path(), &[]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].level, "ui");
        assert_eq!(results[0].resource, "api:packages");
        assert_eq!(results[0].action, "delete");
        assert_eq!(results[0].rule_id, "react-can");
    }

    #[test]
    fn test_scanner_finds_go_permissions() {
        let dir = make_temp_dir();
        write_file(
            dir.path(),
            "handler.go",
            r#"CheckAny(ctx, roles, "api:packages", "read")"#,
        );

        let results = scan(&test_config(), dir.path(), &[]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].level, "api");
    }

    #[test]
    fn test_scanner_skips_non_matching_files() {
        let dir = make_temp_dir();
        write_file(dir.path(), "readme.md", "just docs");

        let results = scan(&test_config(), dir.path(), &[]);
        assert!(results.is_empty());
    }

    #[test]
    fn test_ignore_rules() {
        let dir = make_temp_dir();
        write_file(
            dir.path(),
            "app.tsx",
            r#"<Can object="api:test" action="read">"#,
        );

        let results = scan(&test_config(), dir.path(), &["react-can".to_string()]);
        assert!(results.is_empty());
    }
}
