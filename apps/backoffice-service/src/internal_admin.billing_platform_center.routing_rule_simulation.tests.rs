use sqlx::PgPool;
use uuid::Uuid;

use crate::billing_platform_center_routing_rule_simulation::{
    RoutingRuleSimulationInput, simulate_routing_rule,
};

#[tokio::test]
async fn simulation_matches_highest_priority_active_rule() {
    let Some(pool) = crate::billing_platform_center_actions_test_support::test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !routing_rule_schema_exists(&pool).await {
        eprintln!("skipping test: billing provider routing schema is missing");
        return;
    }

    let customer_type = format!("sim-{}", Uuid::new_v4());
    let broad_rule_id =
        insert_test_rule(&pool, 20, "stripe", &customer_type, 0, 10_000, "active").await;
    let specific_rule_id =
        insert_test_rule(&pool, 10, "mollie", &customer_type, 1_000, 3_000, "active").await;

    let result = simulate_routing_rule(
        &pool,
        RoutingRuleSimulationInput {
            country: Some("zz".to_string()),
            currency: "eur".to_string(),
            payment_method: "card".to_string(),
            customer_type,
            amount_minor: 2_000,
        },
    )
    .await;
    delete_test_rule(&pool, broad_rule_id).await;
    delete_test_rule(&pool, specific_rule_id).await;

    let result = result.expect("simulation should succeed");
    let matched = result.matched_rule.expect("rule should match");
    assert_eq!(result.outcome, "matched");
    assert_eq!(matched.id, specific_rule_id);
    assert_eq!(matched.provider, "mollie");
}

#[tokio::test]
async fn simulation_reports_no_matching_rule() {
    let Some(pool) = crate::billing_platform_center_actions_test_support::test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !routing_rule_schema_exists(&pool).await {
        eprintln!("skipping test: billing provider routing schema is missing");
        return;
    }

    let result = simulate_routing_rule(
        &pool,
        RoutingRuleSimulationInput {
            country: Some("zz".to_string()),
            currency: "eur".to_string(),
            payment_method: "card".to_string(),
            customer_type: format!("sim-{}", Uuid::new_v4()),
            amount_minor: 2_000,
        },
    )
    .await
    .expect("simulation should succeed");

    assert_eq!(result.outcome, "no_matching_rule");
    assert!(result.matched_rule.is_none());
}

#[tokio::test]
async fn simulation_ignores_disabled_rules() {
    let Some(pool) = crate::billing_platform_center_actions_test_support::test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !routing_rule_schema_exists(&pool).await {
        eprintln!("skipping test: billing provider routing schema is missing");
        return;
    }

    let customer_type = format!("sim-{}", Uuid::new_v4());
    let disabled_rule_id =
        insert_test_rule(&pool, 10, "mollie", &customer_type, 0, 10_000, "disabled").await;

    let result = simulate_routing_rule(
        &pool,
        RoutingRuleSimulationInput {
            country: Some("zz".to_string()),
            currency: "eur".to_string(),
            payment_method: "card".to_string(),
            customer_type,
            amount_minor: 2_000,
        },
    )
    .await;
    delete_test_rule(&pool, disabled_rule_id).await;

    let result = result.expect("simulation should succeed");
    assert_eq!(result.outcome, "no_matching_rule");
    assert!(result.matched_rule.is_none());
}

#[tokio::test]
async fn simulation_rejects_negative_amount() {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://localhost/internal_admin_routing_simulation_validation")
        .expect("lazy pool should build");

    let result = simulate_routing_rule(
        &pool,
        RoutingRuleSimulationInput {
            country: Some("fr".to_string()),
            currency: "eur".to_string(),
            payment_method: "card".to_string(),
            customer_type: "b2b".to_string(),
            amount_minor: -1,
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn simulation_normalizes_country_and_currency() {
    let Some(pool) = crate::billing_platform_center_actions_test_support::test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !routing_rule_schema_exists(&pool).await {
        eprintln!("skipping test: billing provider routing schema is missing");
        return;
    }

    let customer_type = format!("sim-{}", Uuid::new_v4());
    let rule_id = insert_test_rule(&pool, 10, "mollie", &customer_type, 0, 10_000, "active").await;

    let result = simulate_routing_rule(
        &pool,
        RoutingRuleSimulationInput {
            country: Some("zz".to_string()),
            currency: "eur".to_string(),
            payment_method: "card".to_string(),
            customer_type,
            amount_minor: 2_000,
        },
    )
    .await;
    delete_test_rule(&pool, rule_id).await;

    let result = result.expect("simulation should succeed");
    let matched = result.matched_rule.expect("rule should match");
    assert_eq!(matched.id, rule_id);
    assert_eq!(result.input.country.as_deref(), Some("ZZ"));
    assert_eq!(result.input.currency, "EUR");
}

#[tokio::test]
async fn simulation_matches_customer_type_case_insensitively() {
    let Some(pool) = crate::billing_platform_center_actions_test_support::test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !routing_rule_schema_exists(&pool).await {
        eprintln!("skipping test: billing provider routing schema is missing");
        return;
    }

    let customer_type = format!("SIM-{}", Uuid::new_v4());
    let rule_id = insert_test_rule(&pool, 10, "mollie", &customer_type, 0, 10_000, "active").await;

    let result = simulate_routing_rule(
        &pool,
        RoutingRuleSimulationInput {
            country: Some("zz".to_string()),
            currency: "eur".to_string(),
            payment_method: "card".to_string(),
            customer_type: customer_type.to_ascii_lowercase(),
            amount_minor: 2_000,
        },
    )
    .await;
    delete_test_rule(&pool, rule_id).await;

    let matched = result
        .expect("simulation should succeed")
        .matched_rule
        .expect("rule should match");
    assert_eq!(matched.id, rule_id);
}

#[tokio::test]
async fn simulation_matches_payment_method_case_insensitively() {
    let Some(pool) = crate::billing_platform_center_actions_test_support::test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !routing_rule_schema_exists(&pool).await {
        eprintln!("skipping test: billing provider routing schema is missing");
        return;
    }

    let customer_type = format!("sim-{}", Uuid::new_v4());
    let rule_id = insert_test_rule(&pool, 10, "mollie", &customer_type, 0, 10_000, "active").await;

    let result = simulate_routing_rule(
        &pool,
        RoutingRuleSimulationInput {
            country: Some("zz".to_string()),
            currency: "eur".to_string(),
            payment_method: "CARD".to_string(),
            customer_type,
            amount_minor: 2_000,
        },
    )
    .await;
    delete_test_rule(&pool, rule_id).await;

    let result = result.expect("simulation should succeed");
    let matched = result.matched_rule.expect("rule should match");
    assert_eq!(matched.id, rule_id);
    assert_eq!(result.input.payment_method, "card");
}

async fn insert_test_rule(
    pool: &PgPool,
    priority: i32,
    provider: &str,
    customer_type: &str,
    min_amount_minor: i64,
    max_amount_minor: i64,
    status: &str,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO billing_provider_routing_rules (
           priority, provider, country, currency, payment_method, customer_type,
           min_amount_minor, max_amount_minor, fallback_enabled, status
         ) VALUES ($1, $2::billing_provider, 'ZZ', 'EUR', 'card', $3, $4, $5, FALSE, $6)
         RETURNING id",
    )
    .bind(priority)
    .bind(provider)
    .bind(customer_type)
    .bind(min_amount_minor)
    .bind(max_amount_minor)
    .bind(status)
    .fetch_one(pool)
    .await
    .expect("test routing rule should insert")
}

async fn delete_test_rule(pool: &PgPool, rule_id: Uuid) {
    sqlx::query("DELETE FROM billing_provider_routing_rules WHERE id = $1")
        .bind(rule_id)
        .execute(pool)
        .await
        .expect("test routing rule should delete");
}

async fn routing_rule_schema_exists(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT to_regclass('public.billing_provider_routing_rules') IS NOT NULL",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}
