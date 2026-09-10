use std::collections::HashSet;
use std::path::Path;

use serde::Deserialize;

use super::TestCase;
use super::registry::KeywordRegistry;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestManifest {
    pub schema_version: u32,
    #[serde(default)]
    pub coverage: Vec<CoverageContract>,
    #[serde(default)]
    pub known_gaps: Vec<KnownGap>,
    #[serde(default)]
    pub policy: Option<ManifestPolicy>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverageContract {
    pub category: String,
    pub lane: String,
    #[serde(default)]
    pub execution: Option<String>,
    #[serde(default)]
    pub iso29119_techniques: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownGap {
    pub categories: Vec<String>,
    #[serde(default)]
    pub release_blocking: Option<bool>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestPolicy {
    #[serde(default)]
    pub silent_skips_allowed: Option<bool>,
    #[serde(default)]
    pub retry_green_allowed: Option<bool>,
}

#[derive(Debug)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    fn new() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn error(&mut self, msg: impl Into<String>) {
        self.valid = false;
        self.errors.push(msg.into());
    }

    fn warning(&mut self, msg: impl Into<String>) {
        self.warnings.push(msg.into());
    }
}

pub fn load_manifest(path: &Path) -> Result<TestManifest, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read manifest {}: {e}", path.display()))?;
    let manifest: TestManifest = serde_json::from_str(&content)
        .map_err(|e| format!("failed to parse manifest {}: {e}", path.display()))?;
    Ok(manifest)
}

pub fn validate_test_case(
    test_case: &TestCase,
    manifest: Option<&TestManifest>,
    registry: &KeywordRegistry,
) -> ValidationResult {
    let mut result = ValidationResult::new();

    validate_keywords(test_case, registry, &mut result);

    if let Some(m) = manifest {
        validate_manifest_schema_version(m, &mut result);
        validate_coverage_tags(test_case, m, &mut result);
        validate_policy(test_case, m, &mut result);
    }

    result
}

fn validate_keywords(
    test_case: &TestCase,
    registry: &KeywordRegistry,
    result: &mut ValidationResult,
) {
    for step in test_case.steps.iter().chain(&test_case.cleanup) {
        if !registry.has(&step.keyword) {
            result.error(format!(
                "step '{}' uses unknown keyword '{}': not registered in keyword registry",
                step.label.as_deref().unwrap_or(&step.keyword),
                step.keyword
            ));
        }
    }
}

fn validate_manifest_schema_version(m: &TestManifest, result: &mut ValidationResult) {
    if m.schema_version != 4 {
        result.error(format!(
            "manifest schema version {} is not supported (expected 4)",
            m.schema_version
        ));
    }
}

fn validate_coverage_tags(test_case: &TestCase, m: &TestManifest, result: &mut ValidationResult) {
    if test_case.tags.is_empty() {
        result.warning(
            "test case has no tags — cannot validate against manifest coverage".to_string(),
        );
        return;
    }

    let manifest_categories: HashSet<&str> =
        m.coverage.iter().map(|c| c.category.as_str()).collect();
    let gap_categories: HashSet<&str> = m
        .known_gaps
        .iter()
        .flat_map(|g| g.categories.iter().map(|c| c.as_str()))
        .collect();

    for tag in &test_case.tags {
        if !manifest_categories.contains(tag.as_str()) && !gap_categories.contains(tag.as_str()) {
            result.warning(format!(
                "test case tag '{}' does not match any manifest coverage category or known gap",
                tag
            ));
        }
    }

    for gap in &m.known_gaps {
        if gap.release_blocking == Some(true) {
            for tag in &test_case.tags {
                if gap.categories.iter().any(|c| c == tag) {
                    result.error(format!(
                        "test case tag '{}' matches blocking gap — this coverage is not yet implemented",
                        tag
                    ));
                }
            }
        }
    }
}

fn validate_policy(test_case: &TestCase, m: &TestManifest, result: &mut ValidationResult) {
    if let Some(ref policy) = m.policy {
        if policy.silent_skips_allowed == Some(false) {
            for step in &test_case.steps {
                if step.keyword.contains("skip") || step.keyword.contains("mock") {
                    result.warning(format!(
                        "policy silentSkipsAllowed=false but step '{}' may be a skip/mock",
                        step.keyword
                    ));
                }
            }
        }
        if policy.retry_green_allowed == Some(false) {
            for step in &test_case.steps {
                if step.keyword.contains("retry") {
                    result.warning(format!(
                        "policy retryGreenAllowed=false but step '{}' may retry on green",
                        step.keyword
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::runner::registry::register_all;

    fn make_registry() -> KeywordRegistry {
        let mut registry = KeywordRegistry::new();
        register_all(&mut registry);
        registry
    }

    fn minimal_test_case(keyword: &str) -> TestCase {
        TestCase {
            id: "TC-VALIDATE-001".to_string(),
            description: None,
            tags: vec![],
            timeout: None,
            cleanup: vec![],
            env: HashMap::new(),
            steps: vec![super::super::Step {
                keyword: keyword.to_string(),
                params: HashMap::new(),
                expect: None,
                label: None,
            }],
        }
    }

    #[test]
    fn valid_keyword_passes() {
        let registry = make_registry();
        let tc = minimal_test_case("data.timestamp");
        let result = validate_test_case(&tc, None, &registry);
        assert!(
            result.valid,
            "expected valid, got errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn unknown_keyword_fails() {
        let registry = make_registry();
        let tc = minimal_test_case("nonexistent.keyword");
        let result = validate_test_case(&tc, None, &registry);
        assert!(!result.valid);
        assert!(result.errors[0].contains("unknown keyword"));
    }

    #[test]
    fn manifest_schema_version_check() {
        let registry = make_registry();
        let tc = minimal_test_case("data.timestamp");
        let manifest = TestManifest {
            schema_version: 99,
            coverage: vec![],
            known_gaps: vec![],
            policy: None,
        };
        let result = validate_test_case(&tc, Some(&manifest), &registry);
        assert!(!result.valid);
        assert!(result.errors[0].contains("schema version"));
    }

    #[test]
    fn blocking_gap_blocks_test() {
        let registry = make_registry();
        let tc = TestCase {
            id: "TC-GAP-001".to_string(),
            description: None,
            tags: vec!["fuzz".to_string()],
            timeout: None,
            cleanup: vec![],
            env: HashMap::new(),
            steps: vec![super::super::Step {
                keyword: "data.timestamp".to_string(),
                params: HashMap::new(),
                expect: None,
                label: None,
            }],
        };
        let manifest = TestManifest {
            schema_version: 4,
            coverage: vec![],
            known_gaps: vec![KnownGap {
                categories: vec!["fuzz".to_string()],
                release_blocking: Some(true),
                description: None,
            }],
            policy: None,
        };
        let result = validate_test_case(&tc, Some(&manifest), &registry);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("blocking gap")));
    }
}
