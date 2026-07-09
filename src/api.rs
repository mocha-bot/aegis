use crate::pattern::ScanResult;
use serde_json::Value;
use std::collections::HashSet;

fn result_key(r: &ScanResult) -> String {
    format!("{}:{}:{}", r.level, r.resource, r.action)
}

pub fn fetch_catalog_api(api_url: &str) -> Result<HashSet<String>, String> {
    let url = format!("{}/api/v1/rbac/catalog", api_url.trim_end_matches('/'));

    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(&url)
        .send()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("API returned status {}", resp.status()));
    }

    let value: Value = resp
        .json()
        .map_err(|e| format!("Failed to parse catalog response: {}", e))?;

    parse_catalog(&value).ok_or_else(|| {
        "Unrecognized catalog schema from API: expected 'data.permissions' or 'permissions'"
            .to_string()
    })
}

pub fn load_catalog_file(path: &str) -> Result<HashSet<String>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read baseline file '{}': {}", path, e))?;

    let value: Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse baseline JSON '{}': {}", path, e))?;

    parse_catalog(&value).ok_or_else(|| {
        format!(
            "Unrecognized baseline schema in '{}': expected 'permissions' or 'data.permissions'",
            path
        )
    })
}

pub fn unregistered(results: &[ScanResult], registered: &HashSet<String>) -> Vec<ScanResult> {
    results
        .iter()
        .filter(|r| !registered.contains(&result_key(r)))
        .cloned()
        .collect()
}

fn parse_catalog(value: &Value) -> Option<HashSet<String>> {
    let perms = value
        .get("permissions")
        .or_else(|| value.get("data").and_then(|d| d.get("permissions")))?
        .as_array()?;

    let mut set = HashSet::new();
    for p in perms {
        if let Some(key) = p.get("key").and_then(Value::as_str) {
            set.insert(key.to_string());
            continue;
        }

        let level = p
            .get("level_key")
            .or_else(|| p.get("levelKey"))
            .and_then(Value::as_str);
        let resource = p
            .get("resource_key")
            .or_else(|| p.get("resourceKey"))
            .and_then(Value::as_str);
        let action = p
            .get("action_key")
            .or_else(|| p.get("actionKey"))
            .and_then(Value::as_str);

        if let (Some(l), Some(r), Some(a)) = (level, resource, action) {
            set.insert(format!("{}:{}:{}", l, r, a));
        }
    }

    Some(set)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(level: &str, resource: &str, action: &str) -> ScanResult {
        ScanResult {
            level: level.into(),
            resource: resource.into(),
            action: action.into(),
            source_file: "test.go".into(),
            source_line: 1,
            rule_id: "test".into(),
        }
    }

    #[test]
    fn parse_catalog_aegis_shape() {
        let value: Value = serde_json::from_str(
            r#"{ "permissions": [
                {"level_key": "api", "resource_key": "api:packages", "action_key": "read"},
                {"level_key": "ui", "resource_key": "api:packages", "action_key": "delete"}
            ]}"#,
        )
        .unwrap();

        let set = parse_catalog(&value).unwrap();
        assert!(set.contains("api:api:packages:read"));
        assert!(set.contains("ui:api:packages:delete"));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn parse_catalog_api_shape() {
        let value: Value = serde_json::from_str(
            r#"{ "data": { "permissions": [
                {"key": "api:packages:read"},
                {"key": "ui:packages:delete"}
            ]}}"#,
        )
        .unwrap();

        let set = parse_catalog(&value).unwrap();
        assert!(set.contains("api:packages:read"));
        assert!(set.contains("ui:packages:delete"));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn parse_catalog_camel_case_keys() {
        let value: Value = serde_json::from_str(
            r#"{ "permissions": [
                {"levelKey": "api", "resourceKey": "packages", "actionKey": "read"}
            ]}"#,
        )
        .unwrap();

        let set = parse_catalog(&value).unwrap();
        assert!(set.contains("api:packages:read"));
    }

    #[test]
    fn parse_catalog_unrecognized_returns_none() {
        let value: Value = serde_json::from_str(r#"{ "something_else": [] }"#).unwrap();
        assert!(parse_catalog(&value).is_none());
    }

    #[test]
    fn load_catalog_file_reads_and_parses() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("aegis_baseline_{}.json", std::process::id()));
        std::fs::write(
            &path,
            r#"{ "permissions": [ {"level_key": "api", "resource_key": "packages", "action_key": "read"} ] }"#,
        )
        .unwrap();

        let set = load_catalog_file(path.to_str().unwrap()).unwrap();
        assert!(set.contains("api:packages:read"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn load_catalog_file_missing_errors() {
        let err = load_catalog_file("/nonexistent/aegis/baseline.json").unwrap_err();
        assert!(err.contains("Failed to read baseline file"));
    }

    #[test]
    fn load_catalog_file_bad_json_errors() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("aegis_bad_{}.json", std::process::id()));
        std::fs::write(&path, "not json").unwrap();

        let err = load_catalog_file(path.to_str().unwrap()).unwrap_err();
        assert!(err.contains("Failed to parse baseline JSON"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn unregistered_filters_registered_permissions() {
        let results = vec![
            result("api", "packages", "read"),
            result("api", "vouchers", "create"),
        ];
        let mut registered = HashSet::new();
        registered.insert("api:packages:read".to_string());

        let missing = unregistered(&results, &registered);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].resource, "vouchers");
    }

    #[test]
    fn unregistered_empty_when_all_registered() {
        let results = vec![result("api", "packages", "read")];
        let mut registered = HashSet::new();
        registered.insert("api:packages:read".to_string());

        assert!(unregistered(&results, &registered).is_empty());
    }
}
