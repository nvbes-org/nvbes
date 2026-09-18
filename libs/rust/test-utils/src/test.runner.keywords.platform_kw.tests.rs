use super::*;
use crate::runner::TestContext;
use crate::runner::variables::VariableContext;

fn ctx() -> TestContext {
    TestContext {
        variables: VariableContext::new(HashMap::new()),
        resources: Default::default(),
    }
}

#[tokio::test]
async fn budget_stage_normal() {
    let mut params = HashMap::new();
    params.insert("spend_cents".to_string(), Value::Number(1000.into()));
    let result = BudgetStage.execute(&ctx(), &params).await.unwrap();
    assert_eq!(result.get("stage").unwrap(), "normal");
    assert_eq!(result.get("within_budget").unwrap(), true);
}

#[tokio::test]
async fn budget_stage_disable_non_essential() {
    let mut params = HashMap::new();
    params.insert("spend_cents".to_string(), Value::Number(2600.into()));
    let result = BudgetStage.execute(&ctx(), &params).await.unwrap();
    assert_eq!(result.get("stage").unwrap(), "disable_non_essential");
    assert_eq!(result.get("within_budget").unwrap(), true);
}

#[tokio::test]
async fn budget_stage_freeze_cost_creation() {
    let mut params = HashMap::new();
    params.insert("spend_cents".to_string(), Value::Number(2900.into()));
    let result = BudgetStage.execute(&ctx(), &params).await.unwrap();
    assert_eq!(result.get("stage").unwrap(), "freeze_cost_creation");
    assert_eq!(result.get("within_budget").unwrap(), true);
}

#[tokio::test]
async fn budget_stage_essential_only() {
    let mut params = HashMap::new();
    params.insert("spend_cents".to_string(), Value::Number(3000.into()));
    let result = BudgetStage.execute(&ctx(), &params).await.unwrap();
    assert_eq!(result.get("stage").unwrap(), "essential_only");
    assert_eq!(result.get("within_budget").unwrap(), false);
}

#[tokio::test]
async fn budget_stage_custom_thresholds() {
    let mut params = HashMap::new();
    params.insert("spend_cents".to_string(), Value::Number(150.into()));
    params.insert(
        "disable_non_essential_cents".to_string(),
        Value::Number(100.into()),
    );
    params.insert(
        "freeze_cost_creation_cents".to_string(),
        Value::Number(200.into()),
    );
    params.insert(
        "essential_only_cents".to_string(),
        Value::Number(300.into()),
    );
    let result = BudgetStage.execute(&ctx(), &params).await.unwrap();
    assert_eq!(result.get("stage").unwrap(), "disable_non_essential");
}

#[tokio::test]
async fn degraded_procedure_identity_unavailable() {
    let mut params = HashMap::new();
    params.insert(
        "scenario".to_string(),
        Value::String("identity_unavailable".to_string()),
    );
    let result = DegradedProcedure.execute(&ctx(), &params).await.unwrap();
    assert_eq!(result.get("title").unwrap(), "Identity Service Unavailable");
    assert_eq!(result.get("severity").unwrap(), "SEV1");
    let checklist = result
        .get("operator_checklist")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(checklist.len(), 4);
    let steps = result
        .get("verification_steps")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(steps.len(), 2);
}

#[tokio::test]
async fn degraded_procedure_all_seven() {
    let scenarios = [
        "identity_unavailable",
        "account_unavailable",
        "stripe_unavailable",
        "email_delayed",
        "trust_risk_unavailable",
        "postgres_in_restore",
        "finops_threshold_exceeded",
    ];
    for scenario in scenarios {
        let mut params = HashMap::new();
        params.insert("scenario".to_string(), Value::String(scenario.to_string()));
        let result = DegradedProcedure.execute(&ctx(), &params).await.unwrap();
        assert!(result.contains_key("title"), "missing title for {scenario}");
        assert!(
            result.contains_key("severity"),
            "missing severity for {scenario}"
        );
        assert!(
            result.contains_key("operator_checklist"),
            "missing checklist for {scenario}"
        );
    }
}

#[tokio::test]
async fn degraded_procedure_unknown_scenario() {
    let mut params = HashMap::new();
    params.insert(
        "scenario".to_string(),
        Value::String("unknown_scenario".to_string()),
    );
    let result = DegradedProcedure.execute(&ctx(), &params).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn budget_stage_missing_params() {
    let params = HashMap::new();
    let result = BudgetStage.execute(&ctx(), &params).await;
    assert!(result.is_err());
}
