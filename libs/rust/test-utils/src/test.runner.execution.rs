use std::collections::HashMap;
use std::time::Duration;

use tokio::time::{Instant, timeout_at};

use super::{
    Step, StepResult, TestCase, TestContext, TestResult, assertions, manifest, registry, variables,
};

pub async fn run_test_case(test_case: &TestCase) -> TestResult {
    run_test_case_with_manifest(test_case, None).await
}

pub async fn run_test_case_with_manifest(
    test_case: &TestCase,
    test_manifest: Option<&manifest::TestManifest>,
) -> TestResult {
    let mut registry = registry::KeywordRegistry::new();
    registry::register_all(&mut registry);
    run_with_registry(test_case, test_manifest, &registry).await
}

pub(super) async fn run_with_registry(
    test_case: &TestCase,
    test_manifest: Option<&manifest::TestManifest>,
    registry: &registry::KeywordRegistry,
) -> TestResult {
    let started = Instant::now();
    let mut steps = Vec::new();
    let validation = manifest::validate_test_case(test_case, test_manifest, registry);
    let duration = parse_timeout(test_case.timeout.as_deref());
    let has_assertion = test_case.steps.iter().any(|step| {
        step.expect.as_ref().is_some_and(|expect| {
            expect.status.is_some()
                || expect.error.as_ref().is_some_and(|s| !s.is_empty())
                || expect.body.as_ref().is_some_and(|body| !body.is_empty())
                || expect
                    .contains
                    .as_ref()
                    .is_some_and(|body| !body.is_empty())
        })
    });
    let mut labels = std::collections::HashSet::new();
    let unique_labels = test_case
        .steps
        .iter()
        .chain(&test_case.cleanup)
        .filter_map(|step| step.label.as_ref())
        .all(|label| labels.insert(label));
    let environment = test_case
        .env
        .get("NVBES_ENV")
        .cloned()
        .or_else(|| std::env::var("NVBES_ENV").ok());
    let unsafe_target = test_case.env.iter().any(|(key, value)| {
        (key.ends_with("_URL") || key.ends_with("_ENDPOINT"))
            && crate::environment::validate_loopback_url(value, "scenario target").is_err()
    });
    if !validation.valid
        || test_case.id.trim().is_empty()
        || !has_assertion
        || !unique_labels
        || duration.is_none()
        || unsafe_target
        || crate::environment::validate_test_environment(environment.as_deref()).is_err()
    {
        steps.push(failed(
            "validation",
            "invalid scenario, environment, timeout or manifest",
            0,
        ));
        return TestResult {
            test_case: test_case.clone(),
            steps,
            passed: false,
            total_duration_ms: 0,
        };
    }
    let deadline = started + duration.expect("validated timeout");
    let mut context = TestContext {
        variables: variables::VariableContext::new(test_case.env.clone()),
        resources: Default::default(),
    };
    let mut passed = true;
    for step in &test_case.steps {
        let result = execute(step, registry, &context, deadline).await;
        let success = result.passed;
        context.variables.push_output(result.output.clone());
        steps.push(result);
        if !success {
            passed = false;
            break;
        }
    }
    // Cleanup has a separate bounded budget and runs even after a main-step timeout.
    let cleanup_deadline = Instant::now() + Duration::from_secs(10);
    for step in &test_case.cleanup {
        let result = execute(step, registry, &context, cleanup_deadline).await;
        passed &= result.passed;
        steps.push(result);
    }
    let resources = timeout_at(Instant::now() + Duration::from_secs(10), async {
        context.resources.lock().await.cleanup().await
    })
    .await;
    if !matches!(resources, Ok(Ok(()))) {
        passed = false;
        steps.push(failed(
            "infra.cleanup",
            "owned resource cleanup failed or timed out",
            0,
        ));
    }
    TestResult {
        test_case: test_case.clone(),
        steps,
        passed,
        total_duration_ms: started.elapsed().as_millis() as u64,
    }
}

fn parse_timeout(value: Option<&str>) -> Option<Duration> {
    let value = value.unwrap_or("30s");
    let (number, multiplier) = if let Some(number) = value.strip_suffix("ms") {
        (number, 1)
    } else {
        (value.strip_suffix('s')?, 1000)
    };
    let milliseconds = number.parse::<u64>().ok()?.checked_mul(multiplier)?;
    (milliseconds > 0 && milliseconds <= 3_600_000).then(|| Duration::from_millis(milliseconds))
}

fn failed(keyword: &str, error: &str, duration_ms: u64) -> StepResult {
    StepResult {
        keyword: keyword.to_string(),
        label: None,
        output: HashMap::new(),
        passed: false,
        errors: vec![error.to_string()],
        duration_ms,
    }
}

async fn execute(
    step: &Step,
    registry: &registry::KeywordRegistry,
    context: &TestContext,
    deadline: Instant,
) -> StepResult {
    let started = Instant::now();
    let Some(keyword) = registry.get(&step.keyword) else {
        return failed(&step.keyword, "unknown keyword", 0);
    };
    let params: HashMap<_, _> = step
        .params
        .iter()
        .map(|(key, value)| {
            (
                key.clone(),
                variables::substitute_value(value, &context.variables),
            )
        })
        .collect();
    if params
        .values()
        .any(|value| value.to_string().contains("${"))
        || step.expect.as_ref().is_some_and(|expect| {
            expect.body.as_ref().is_some_and(|body| {
                body.values().any(|value| {
                    variables::substitute_value(value, &context.variables)
                        .to_string()
                        .contains("${")
                })
            }) || expect.contains.as_ref().is_some_and(|body| {
                body.values()
                    .any(|value| variables::substitute(value, &context.variables).contains("${"))
            })
        })
    {
        return failed(&step.keyword, "unresolved variable", 0);
    }
    for (key, value) in &params {
        if (key == "url" || key.ends_with("_url"))
            && value.as_str().is_some_and(|url| {
                crate::environment::validate_loopback_url(url, "test target").is_err()
            })
        {
            return failed(&step.keyword, "non-local test target", 0);
        }
    }
    let execution = timeout_at(deadline, keyword.execute(context, &params)).await;
    let elapsed = started.elapsed().as_millis() as u64;
    let (output, error) = match execution {
        Err(_) => return failed(&step.keyword, "scenario timeout", elapsed),
        Ok(Err(error)) => (HashMap::new(), Some(error.to_string())),
        Ok(Ok(output)) => (output, None),
    };
    let status = output.get("status").and_then(|value| {
        value
            .as_u64()
            .and_then(|number| u16::try_from(number).ok())
            .or_else(|| (value.as_str() == Some("ok")).then_some(200))
    });
    let passed = match &step.expect {
        Some(expect) => assertions::validate_assertions(
            expect,
            status,
            &output,
            error.as_deref(),
            &context.variables,
        )
        .is_ok(),
        None => error.is_none() && status.is_none_or(|value| value < 400),
    };
    // Errors can contain credentials or assertion values; only stable categories are public.
    StepResult {
        keyword: step.keyword.clone(),
        label: step.label.clone(),
        output,
        passed,
        errors: if passed {
            vec![]
        } else {
            vec!["keyword execution or assertion failed".into()]
        },
        duration_ms: elapsed,
    }
}

#[cfg(test)]
#[path = "test.runner.execution.tests.rs"]
mod tests;
