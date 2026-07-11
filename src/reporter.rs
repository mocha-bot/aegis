use crate::pattern::ScanResult;

pub fn report_table(results: &[ScanResult]) -> String {
    if results.is_empty() {
        return "No permissions found.\n".to_string();
    }

    let mut out = String::new();
    out.push_str(&format!(
        "{:<10} {:<30} {:<12} {:<40} {:>6}  {}\n",
        "LEVEL", "RESOURCE", "ACTION", "FILE", "LINE", "RULE"
    ));
    out.push_str(&"-".repeat(120));
    out.push('\n');

    for r in results {
        out.push_str(&format!(
            "{:<10} {:<30} {:<12} {:<40} {:>6}  {}\n",
            r.level, r.resource, r.action, r.source_file, r.source_line, r.rule_id
        ));
    }

    out.push_str(&format!("\n{} permissions found.\n", results.len()));
    out
}

pub fn report_csv(results: &[ScanResult]) -> String {
    let mut out = String::from("level,resource,action,source_file,source_line,rule_id\n");
    for r in results {
        out.push_str(&format!(
            "{},{},{},{},{},{}\n",
            r.level, r.resource, r.action, r.source_file, r.source_line, r.rule_id
        ));
    }
    out
}

pub fn report_json(results: &[ScanResult]) -> String {
    serde_json::to_string_pretty(results).unwrap_or_else(|_| "[]".to_string())
}

pub fn report_catalog_json(results: &[ScanResult]) -> String {
    let mut seen = std::collections::HashSet::new();
    let mut entries: Vec<(String, serde_json::Value)> = Vec::new();

    for r in results {
        let key = format!("{}:{}:{}", r.level, r.resource, r.action);
        if seen.insert(key.clone()) {
            entries.push((
                key,
                serde_json::json!({
                    "level_key": r.level,
                    "resource_key": r.resource,
                    "action_key": r.action,
                }),
            ));
        }
    }

    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let perms: Vec<serde_json::Value> = entries.into_iter().map(|(_, v)| v).collect();

    serde_json::to_string_pretty(&serde_json::json!({
        "permissions": perms
    }))
    .unwrap_or_else(|_| "{}".to_string())
}

const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const RESET: &str = "\x1b[0m";

pub fn report_diff(added: &[ScanResult], removed: &[String], color: bool) -> String {
    let paint = |line: String, code: &str| {
        if color {
            format!("{}{}{}", code, line, RESET)
        } else {
            line
        }
    };

    let mut out = format!("{} permission change(s):\n", added.len() + removed.len());

    let mut added_lines: Vec<String> = added
        .iter()
        .map(|r| {
            format!(
                "+ {}:{}:{}   {}:{}",
                r.level, r.resource, r.action, r.source_file, r.source_line
            )
        })
        .collect();
    added_lines.sort();
    for line in added_lines {
        out.push_str(&paint(line, GREEN));
        out.push('\n');
    }

    for key in removed {
        out.push_str(&paint(format!("- {}", key), RED));
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_results() -> Vec<ScanResult> {
        vec![
            ScanResult {
                level: "ui".to_string(),
                resource: "api:packages".to_string(),
                action: "delete".to_string(),
                source_file: "src/App.tsx".to_string(),
                source_line: 42,
                rule_id: "react-can".to_string(),
            },
            ScanResult {
                level: "api".to_string(),
                resource: "api:packages".to_string(),
                action: "read".to_string(),
                source_file: "handler.go".to_string(),
                source_line: 15,
                rule_id: "go-check".to_string(),
            },
        ]
    }

    #[test]
    fn test_table_output_contains_data() {
        let out = report_table(&sample_results());
        assert!(out.contains("api:packages"));
        assert!(out.contains("delete"));
        assert!(out.contains("2 permissions found"));
    }

    #[test]
    fn test_table_empty() {
        let out = report_table(&[]);
        assert!(out.contains("No permissions found"));
    }

    #[test]
    fn test_csv_output() {
        let out = report_csv(&sample_results());
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 3); // header + 2 rows
        assert!(lines[0].starts_with("level,resource,action"));
        assert!(lines[1].contains("api:packages"));
    }

    #[test]
    fn test_json_output() {
        let out = report_json(&sample_results());
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed.len(), 2);
    }

    #[test]
    fn test_catalog_json_dedup() {
        let mut results = sample_results();
        // Add a duplicate (same level:resource:action but different file)
        results.push(ScanResult {
            level: "api".to_string(),
            resource: "api:packages".to_string(),
            action: "read".to_string(),
            source_file: "other.go".to_string(),
            source_line: 99,
            rule_id: "go-check".to_string(),
        });

        let out = report_catalog_json(&results);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        let perms = parsed["permissions"].as_array().unwrap();
        // Deduped — only 2 unique permission keys
        assert_eq!(perms.len(), 2);
    }

    #[test]
    fn test_catalog_json_sorted() {
        let results = vec![
            ScanResult {
                level: "api".to_string(),
                resource: "zebra".to_string(),
                action: "read".to_string(),
                source_file: "z.go".to_string(),
                source_line: 1,
                rule_id: "r".to_string(),
            },
            ScanResult {
                level: "api".to_string(),
                resource: "apple".to_string(),
                action: "read".to_string(),
                source_file: "a.go".to_string(),
                source_line: 2,
                rule_id: "r".to_string(),
            },
        ];

        let out = report_catalog_json(&results);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        let perms = parsed["permissions"].as_array().unwrap();
        assert_eq!(perms[0]["resource_key"], "apple");
        assert_eq!(perms[1]["resource_key"], "zebra");
    }

    fn added_result(resource: &str, line: usize) -> ScanResult {
        ScanResult {
            level: "api".to_string(),
            resource: resource.to_string(),
            action: "create".to_string(),
            source_file: "handler.go".to_string(),
            source_line: line,
            rule_id: "go-check".to_string(),
        }
    }

    #[test]
    fn test_report_diff_added_and_removed() {
        let added = vec![added_result("vouchers", 5)];
        let removed = vec!["api:packages:read".to_string()];

        let out = report_diff(&added, &removed, false);
        assert!(out.contains("2 permission change(s):"));
        assert!(out.contains("+ api:vouchers:create"));
        assert!(out.contains("handler.go:5"));
        assert!(out.contains("- api:packages:read"));
    }

    #[test]
    fn test_report_diff_added_only() {
        let added = vec![added_result("vouchers", 5)];
        let out = report_diff(&added, &[], false);
        assert!(out.contains("1 permission change(s):"));
        assert!(out.contains("+ api:vouchers:create"));
        assert!(!out.contains("- "));
    }

    #[test]
    fn test_report_diff_removed_only() {
        let removed = vec!["api:packages:read".to_string()];
        let out = report_diff(&[], &removed, false);
        assert!(out.contains("1 permission change(s):"));
        assert!(out.contains("- api:packages:read"));
        assert!(!out.contains("+ "));
    }

    #[test]
    fn test_report_diff_added_sorted_by_key() {
        let added = vec![added_result("zebra", 1), added_result("apple", 2)];
        let out = report_diff(&added, &[], false);
        let apple = out.find("apple").unwrap();
        let zebra = out.find("zebra").unwrap();
        assert!(apple < zebra);
    }

    #[test]
    fn test_report_diff_no_color_has_no_ansi() {
        let added = vec![added_result("vouchers", 5)];
        let removed = vec!["api:packages:read".to_string()];
        let out = report_diff(&added, &removed, false);
        assert!(!out.contains('\x1b'));
    }

    #[test]
    fn test_report_diff_color_wraps_lines() {
        let added = vec![added_result("vouchers", 5)];
        let removed = vec!["api:packages:read".to_string()];
        let out = report_diff(&added, &removed, true);
        // green + line, red - line, reset after each
        assert!(out.contains("\x1b[32m+ api:vouchers:create"));
        assert!(out.contains("\x1b[31m- api:packages:read"));
        assert!(out.contains("\x1b[0m"));
        // summary line stays uncolored
        assert!(out.starts_with("2 permission change(s):"));
    }
}
