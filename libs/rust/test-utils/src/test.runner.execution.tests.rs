use super::*;
use async_trait::async_trait;
use registry::{Keyword, KeywordError, KeywordRegistry};
use serde_json::Value;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct Echo;
#[async_trait]
impl Keyword for Echo {
    async fn execute(
        &self,
        context: &TestContext,
        params: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        let mut output = params.clone();
        output.insert(
            "prior_steps".into(),
            Value::from(context.variables.step_outputs.len()),
        );
        Ok(output)
    }
}

struct Counter(Arc<AtomicUsize>);
#[async_trait]
impl Keyword for Counter {
    async fn execute(
        &self,
        _: &TestContext,
        _: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(HashMap::new())
    }
}

struct Slow;
#[async_trait]
impl Keyword for Slow {
    async fn execute(
        &self,
        _: &TestContext,
        _: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, KeywordError> {
        tokio::time::sleep(Duration::from_secs(60)).await;
        Ok(HashMap::new())
    }
}

fn registry() -> (KeywordRegistry, Arc<AtomicUsize>) {
    let counter = Arc::new(AtomicUsize::new(0));
    let mut registry = KeywordRegistry::new();
    registry.register("echo", Box::new(Echo));
    registry.register("count", Box::new(Counter(counter.clone())));
    registry.register("slow", Box::new(Slow));
    (registry, counter)
}

#[tokio::test]
async fn propagates_outputs_to_both_substitution_and_keyword_context() {
    let (registry, _) = registry();
    let case = super::super::parse_test_case(
        r#"
id: propagation
steps:
  - keyword: echo
    params: {value: first}
  - keyword: echo
    params: {value: '${LAST.value}'}
    expect:
      body: {value: first, prior_steps: 1}
"#,
    )
    .unwrap();
    assert!(run_with_registry(&case, None, &registry).await.passed);
}

#[tokio::test]
async fn stops_dependent_steps_and_runs_cleanup_after_assertion_failure() {
    let (registry, counter) = registry();
    let case = super::super::parse_test_case(
        r#"
id: failure
steps:
  - keyword: echo
    params: {value: private-token}
    expect: {body: {value: different}}
  - keyword: count
cleanup:
  - keyword: count
"#,
    )
    .unwrap();
    let result = run_with_registry(&case, None, &registry).await;
    assert!(!result.passed);
    assert_eq!(counter.load(Ordering::SeqCst), 1);
    assert!(!result.steps[0].errors.join(" ").contains("private-token"));
}

#[tokio::test]
async fn timeout_still_runs_cleanup_with_a_separate_deadline() {
    let (registry, counter) = registry();
    let case = super::super::parse_test_case(
        r#"
id: timeout
timeout: 5ms
steps:
  - keyword: slow
    expect: {status: 200}
  - keyword: count
cleanup:
  - keyword: count
"#,
    )
    .unwrap();
    let result = run_with_registry(&case, None, &registry).await;
    assert!(!result.passed);
    assert_eq!(result.steps[0].errors, ["scenario timeout"]);
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn rejects_invalid_scenarios_before_any_side_effect() {
    for tail in [
        "  - keyword: missing\n    expect: {status: 200}",
        "  - keyword: echo\n    expect: {}",
    ] {
        let (registry, counter) = registry();
        let case = super::super::parse_test_case(&format!(
            "id: invalid\nsteps:\n  - keyword: count\n{tail}\n"
        ))
        .unwrap();
        assert!(!run_with_registry(&case, None, &registry).await.passed);
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn missing_variable_and_remote_target_fail_without_echoing_credentials() {
    for params in [
        "{value: '${ABSENT_RUNNER_TEST_VALUE}'}",
        "{url: 'https://secret:password@example.com'}",
    ] {
        let (registry, _) = registry();
        let case = super::super::parse_test_case(&format!("id: invalid\nsteps:\n  - keyword: echo\n    params: {params}\n    expect: {{body: {{value: '*'}}}}\n")).unwrap();
        let result = run_with_registry(&case, None, &registry).await;
        assert!(!result.passed);
        assert!(!result.steps[0].errors.join(" ").contains("password"));
    }
}

#[test]
fn timeout_and_status_numbers_do_not_wrap() {
    for value in ["0s", "-1s", "3601s", "18446744073709551615s", "five"] {
        assert!(parse_timeout(Some(value)).is_none());
    }
    assert!(
        super::super::parse_test_case(
            "id: wrap\nsteps:\n  - keyword: echo\n    expect: {status: 65736}\n"
        )
        .is_err()
    );
}
