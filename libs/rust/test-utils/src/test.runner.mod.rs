#[path = "test.runner.assertions.rs"]
pub mod assertions;
#[path = "test.runner.execution.rs"]
mod execution;
#[path = "test.runner.keywords.mod.rs"]
pub mod keywords;
#[path = "test.runner.manifest.rs"]
pub mod manifest;
#[path = "test.runner.registry.rs"]
pub mod registry;
#[path = "test.runner.resources.rs"]
pub mod resources;
#[path = "test.runner.transport.rs"]
pub mod transport;
#[path = "test.runner.variables.rs"]
pub mod variables;

use std::collections::HashMap;

use assertions::ExpectBlock;
use serde::Deserialize;
use serde_json::Value;

pub use registry::KeywordError;
pub use variables::VariableContext;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestCase {
    pub id: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub timeout: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    pub steps: Vec<Step>,
    #[serde(default)]
    pub cleanup: Vec<Step>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub keyword: String,
    #[serde(default)]
    pub params: HashMap<String, Value>,
    #[serde(default)]
    pub expect: Option<ExpectBlock>,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug)]
pub struct StepResult {
    pub keyword: String,
    pub label: Option<String>,
    pub output: HashMap<String, Value>,
    pub passed: bool,
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

#[derive(Debug)]
pub struct TestResult {
    pub test_case: TestCase,
    pub steps: Vec<StepResult>,
    pub passed: bool,
    pub total_duration_ms: u64,
}

pub struct TestContext {
    pub variables: VariableContext,
    pub resources: tokio::sync::Mutex<resources::Resources>,
}

pub use execution::{run_test_case, run_test_case_with_manifest};

pub fn parse_test_case(yaml: &str) -> Result<TestCase, serde_yaml_ng::Error> {
    serde_yaml_ng::from_str(yaml)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_test_case() {
        let yaml = r#"
id: TC-TEST-001
steps:
  - keyword: data.timestamp
"#;
        let tc = parse_test_case(yaml).unwrap();
        assert_eq!(tc.id, "TC-TEST-001");
        assert_eq!(tc.steps.len(), 1);
        assert_eq!(tc.steps[0].keyword, "data.timestamp");
    }

    #[test]
    fn parses_full_test_case() {
        let yaml = r#"
id: TC-FULL-001
description: "Full test case"
tags: [integration, data]
timeout: 30s
env:
  NVBES_ENV: test
steps:
  - keyword: env.validate_test
  - keyword: data.set_var
    params:
      key: "test:123"
      value: "hello"
    expect:
      status: ok
  - keyword: data.get_var
    params:
      key: "${LAST.key}"
    label: "get after set"
    expect:
      body:
        value: "hello"
        found: true
"#;
        let tc = parse_test_case(yaml).unwrap();
        assert_eq!(tc.id, "TC-FULL-001");
        assert_eq!(tc.tags, vec!["integration", "data"]);
        assert_eq!(tc.env.get("NVBES_ENV").unwrap(), "test");
        assert_eq!(tc.steps.len(), 3);
        assert!(tc.steps[1].expect.is_some());
    }

    #[tokio::test]
    async fn runs_timestamp_keyword() {
        let yaml = r#"
id: TC-TS-001
steps:
  - keyword: data.timestamp
    expect:
      body:
        timestamp: "*"
"#;
        let tc = parse_test_case(yaml).unwrap();
        let result = run_test_case(&tc).await;
        assert!(result.passed, "test failed: {:?}", result.steps[0].errors);
    }
}
