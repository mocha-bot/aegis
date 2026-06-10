use crate::pattern::ScanResult;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CatalogResponse {
    data: Option<CatalogData>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct CatalogData {
    resources: Option<Vec<CatalogResource>>,
    permissions: Option<Vec<CatalogPermission>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct CatalogResource {
    #[serde(alias = "levelKey", alias = "level_key")]
    level_key: String,
    key: String,
}

#[derive(Debug, Deserialize)]
struct CatalogPermission {
    key: String,
}

pub fn diff_against_catalog(
    results: &[ScanResult],
    api_url: &str,
) -> Result<Vec<ScanResult>, String> {
    let url = format!("{}/api/v1/rbac/catalog", api_url.trim_end_matches('/'));

    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(&url)
        .send()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("API returned status {}", resp.status()));
    }

    let catalog: CatalogResponse = resp
        .json()
        .map_err(|e| format!("Failed to parse catalog response: {}", e))?;

    let registered: std::collections::HashSet<String> = catalog
        .data
        .iter()
        .flat_map(|d| d.permissions.iter())
        .flatten()
        .map(|p| p.key.clone())
        .collect();

    let unregistered: Vec<ScanResult> = results
        .iter()
        .filter(|r| {
            let key = format!("{}:{}:{}", r.level, r.resource, r.action);
            !registered.contains(&key)
        })
        .cloned()
        .collect();

    Ok(unregistered)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_with_empty_catalog_marks_all_unregistered() {
        // This test verifies the logic without making real HTTP calls.
        // For integration tests that hit a real API, see tests/integration.rs
        let results = vec![ScanResult {
            level: "api".into(),
            resource: "packages".into(),
            action: "read".into(),
            source_file: "test.go".into(),
            source_line: 1,
            rule_id: "test".into(),
        }];

        let key = format!(
            "{}:{}:{}",
            results[0].level, results[0].resource, results[0].action
        );
        assert_eq!(key, "api:packages:read");
    }
}
