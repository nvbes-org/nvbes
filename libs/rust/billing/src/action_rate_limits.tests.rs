use nvbes_core::limiter::{RateLimiter, SCHEMA};
use sqlx::PgPool;
use uuid::Uuid;

use super::enforce_billing_action_rate_limits;

#[sqlx::test]
async fn enforce_billing_action_rate_limits_checks_workspace_and_actor(pool: PgPool) {
    sqlx::raw_sql(SCHEMA).execute(&pool).await.unwrap();
    let limiter = RateLimiter::new(pool);
    let workspace_action = format!("checkout-{}", Uuid::new_v4());
    let user_action = format!("portal-{}", Uuid::new_v4());
    let workspace_id = Uuid::new_v4();
    let actor_id = Uuid::new_v4();

    for _ in 0..4 {
        enforce_billing_action_rate_limits(&limiter, workspace_id, actor_id, &workspace_action)
            .await
            .expect("workspace bucket allows four hits");
    }
    let workspace_limited =
        enforce_billing_action_rate_limits(&limiter, workspace_id, actor_id, &workspace_action)
            .await
            .expect_err("fifth workspace hit is rate limited");
    assert_eq!(workspace_limited.code, "rate_limited");

    for index in 0..6 {
        let workspace = Uuid::new_v4();
        enforce_billing_action_rate_limits(&limiter, workspace, actor_id, &user_action)
            .await
            .unwrap_or_else(|error| panic!("user bucket hit {index} should succeed: {error:?}"));
    }
    let user_limited =
        enforce_billing_action_rate_limits(&limiter, Uuid::new_v4(), actor_id, &user_action)
            .await
            .expect_err("seventh user hit is rate limited");
    assert_eq!(user_limited.code, "rate_limited");
}
