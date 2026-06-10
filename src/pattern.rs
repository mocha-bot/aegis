use regex::Regex;
use serde::Serialize;
use crate::config::{CaptureMode, PatternDef};

#[derive(Debug, Clone)]
pub struct CompiledPattern {
    pub rule_id: String,
    pub level: String,
    pub regex: Regex,
    pub capture_mode: CaptureMode,
    pub sub_regex: Option<Regex>,
}

impl CompiledPattern {
    pub fn compile(rule_id: &str, pattern: &PatternDef) -> Result<Self, regex::Error> {
        let regex = Regex::new(&pattern.regex)?;
        let sub_regex = match &pattern.sub_pattern {
            Some(sp) => Some(Regex::new(sp)?),
            None => None,
        };
        Ok(CompiledPattern {
            rule_id: rule_id.to_string(),
            level: pattern.level.clone(),
            regex,
            capture_mode: pattern.capture_mode.clone(),
            sub_regex,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScanResult {
    pub level: String,
    pub resource: String,
    pub action: String,
    pub source_file: String,
    pub source_line: usize,
    pub rule_id: String,
}

pub fn extract_matches(
    content: &str,
    line_num: usize,
    compiled: &CompiledPattern,
    file_path: &str,
) -> Vec<ScanResult> {
    let mut results = Vec::new();

    for caps in compiled.regex.captures_iter(content) {
        match compiled.capture_mode {
            CaptureMode::Single => {
                let object = caps.name("object").map(|m| m.as_str().to_string());
                let action = caps.name("action").map(|m| m.as_str().to_string());
                let cap_level = caps.name("level").map(|m| m.as_str().to_string());

                if let (Some(object), Some(action)) = (object, action) {
                    let level = resolve_level(&compiled.level, cap_level);
                    results.push(ScanResult {
                        level,
                        resource: object,
                        action,
                        source_file: file_path.to_string(),
                        source_line: line_num,
                        rule_id: compiled.rule_id.clone(),
                    });
                }
            }
            CaptureMode::Repeated => {
                if let Some(sub_re) = &compiled.sub_regex {
                    // The outer regex captures a block; apply sub_regex to find all pairs
                    let block = caps.get(0).map(|m| m.as_str()).unwrap_or("");
                    for sub_caps in sub_re.captures_iter(block) {
                        let object = sub_caps.name("object").map(|m| m.as_str().to_string());
                        let action = sub_caps.name("action").map(|m| m.as_str().to_string());
                        let cap_level = sub_caps.name("level").map(|m| m.as_str().to_string());

                        if let (Some(object), Some(action)) = (object, action) {
                            let level = resolve_level(&compiled.level, cap_level);
                            results.push(ScanResult {
                                level,
                                resource: object,
                                action,
                                source_file: file_path.to_string(),
                                source_line: line_num,
                                rule_id: compiled.rule_id.clone(),
                            });
                        }
                    }
                }
            }
        }
    }

    results
}

fn resolve_level(default: &str, captured: Option<String>) -> String {
    match (default, captured) {
        ("$level", Some(cap)) => cap,
        (other, _) => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compile_single(regex: &str, level: &str) -> CompiledPattern {
        CompiledPattern::compile(
            "test",
            &PatternDef {
                regex: regex.to_string(),
                level: level.to_string(),
                capture_mode: CaptureMode::Single,
                sub_pattern: None,
            },
        )
        .unwrap()
    }

    #[test]
    fn test_single_match_can_component() {
        let compiled = compile_single(
            r#"<Can\s+object="(?P<object>[^"]+)"\s+action="(?P<action>[^"]+)"#,
            "ui",
        );
        let content = r#"<Can object="api:packages" action="delete">"#;
        let results = extract_matches(content, 42, &compiled, "App.tsx");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].level, "ui");
        assert_eq!(results[0].resource, "api:packages");
        assert_eq!(results[0].action, "delete");
        assert_eq!(results[0].source_line, 42);
        assert_eq!(results[0].rule_id, "test");
    }

    #[test]
    fn test_single_match_use_permission() {
        let compiled = compile_single(
            r#"usePermission\(["'](?P<object>[^"']+)["'],\s*["'](?P<action>[^"']+)["']"#,
            "ui",
        );
        let content = r#"usePermission("api:packages", "read")"#;
        let results = extract_matches(content, 10, &compiled, "hook.ts");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].resource, "api:packages");
        assert_eq!(results[0].action, "read");
    }

    #[test]
    fn test_single_match_go_check_any() {
        let compiled = compile_single(
            r#"CheckAny\(.*?,\s*"(?P<object>[^"]+)",\s*"(?P<action>[^"]+)"\)"#,
            "api",
        );
        let content = r#"CheckAny(ctx, roles, "api:packages", "read")"#;
        let results = extract_matches(content, 15, &compiled, "handler.go");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].level, "api");
        assert_eq!(results[0].resource, "api:packages");
    }

    #[test]
    fn test_repeated_mode_any_block() {
        let pattern = PatternDef {
            regex: r#"any=\{\[\s*((?:\{[^}]+\},?\s*)+)\]"#.to_string(),
            level: "ui".to_string(),
            capture_mode: CaptureMode::Repeated,
            sub_pattern: Some(
                r#"\{object:\s*"(?P<object>[^"]+)",\s*action:\s*"(?P<action>[^"]+)"\}"#.to_string(),
            ),
        };
        let compiled = CompiledPattern::compile("test", &pattern).unwrap();
        let content = r#"<Can any={[{object: "api:packages", action: "delete"}, {object: "api:transactions", action: "read"}]}>"#;
        let results = extract_matches(content, 99, &compiled, "Page.tsx");

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].resource, "api:packages");
        assert_eq!(results[0].action, "delete");
        assert_eq!(results[1].resource, "api:transactions");
        assert_eq!(results[1].action, "read");
    }

    #[test]
    fn test_repeated_mode_all_block() {
        let pattern = PatternDef {
            regex: r#"all=\{\[\s*((?:\{[^}]+\},?\s*)+)\]"#.to_string(),
            level: "ui".to_string(),
            capture_mode: CaptureMode::Repeated,
            sub_pattern: Some(
                r#"object:\s*"(?P<object>[^"]+)",\s*action:\s*"(?P<action>[^"]+)"\}"#.to_string(),
            ),
        };
        let compiled = CompiledPattern::compile("test", &pattern).unwrap();
        let content = r#"all={[{object: "api:packages", action: "delete"}, {object: "api:packages", action: "read"}]}"#;
        let results = extract_matches(content, 42, &compiled, "Page.tsx");

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].source_line, 42);
        assert_eq!(results[1].source_line, 42);
        // Both share same source line — correct, they're in the same component
    }

    #[test]
    fn test_annotation_comment() {
        let compiled = compile_single(
            r#"@rbac\s+(?P<level>\w+):(?P<object>[\w.-]+):(?P<action>\w+)"#,
            "$level",
        );
        let content = "// @rbac api:webhooks:create";
        let results = extract_matches(content, 1, &compiled, "main.go");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].level, "api");
        assert_eq!(results[0].resource, "webhooks");
        assert_eq!(results[0].action, "create");
    }

    #[test]
    fn test_no_match_returns_empty() {
        let compiled = compile_single(r#"<Can\s+object="(?P<object>[^"]+)"# , "ui");
        let content = "regular text without permissions";
        let results = extract_matches(content, 1, &compiled, "file.ts");
        assert!(results.is_empty());
    }

    #[test]
    fn test_action_before_object() {
        let compiled = compile_single(
            r#"<Can\s+action="(?P<action>[^"]+)"\s+object="(?P<object>[^"]+)"#,
            "ui",
        );
        let content = r#"<Can action="delete" object="api:packages">"#;
        let results = extract_matches(content, 1, &compiled, "test.tsx");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action, "delete");
        assert_eq!(results[0].resource, "api:packages");
    }
}
