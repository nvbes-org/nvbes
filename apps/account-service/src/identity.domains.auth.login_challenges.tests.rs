#[cfg(test)]
use super::storage::set_challenge;
#[cfg(test)]
use super::{
    CachedLoginChallenge, CreateLoginChallengeInput, LoginChallenge, fetch_active_challenge,
    get_challenge, prune_expired_challenges, record_failed_attempt, replace_challenge,
};
#[cfg(test)]
use crate::domains::auth::state::create_state;
#[cfg(test)]
use chrono::Utc;
#[cfg(test)]
use sqlx::PgPool;
#[cfg(test)]
use std::env;
#[cfg(test)]
use uuid::Uuid;

#[cfg(test)]
fn test_pool() -> PgPool {
    crate::test_support::shared_test_pool()
}

#[cfg(test)]
#[test]
fn login_challenge_shape_keeps_factor_policy_fields() {
    let challenge = LoginChallenge {
        id: Uuid::nil(),
        auth_state_id: Uuid::nil(),
        principal_id: None,
        metadata: serde_json::json!({}),
        allowed_factor_types: vec!["webauthn".to_string()],
        failed_attempts: 0,
    };

    assert_eq!(challenge.allowed_factor_types, vec!["webauthn"]);
    assert_eq!(challenge.failed_attempts, 0);
}

#[cfg(test)]
#[tokio::test]
async fn replace_challenge_prunes_expired_and_keeps_active_flow_unique() {
    let pool = test_pool();
    let redis = crate::test_support::test_redis_pool().await;
    crate::test_support::ensure_test_database(&pool).await;
    unsafe {
        env::set_var("NVBES_ENV", "development");
        env::set_var(
            "NVBES_WORKSPACE_ROOT",
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .nth(2)
                .expect("workspace root"),
        );
    }
    let auth_state_id = create_state(&redis, None, "challenge@example.com", "mfa", None)
        .await
        .expect("auth state should be created");
    let principal_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO principals (id, tenant_id, principal_kind, status, display_name, created_at, updated_at)
        VALUES ($1, NULL, 'human', 'active', 'Challenge Tester', $2, $2)
        "#,
    )
    .bind(principal_id)
    .bind(Utc::now())
    .execute(&pool)
    .await
    .expect("principal should be created");

    let expired_id = Uuid::new_v4();
    let expired = CachedLoginChallenge {
        id: expired_id,
        auth_state_id,
        principal_id: Some(principal_id),
        tenant_id: None,
        workspace_id: None,
        purpose: "webauthn_login".to_string(),
        required_level: "aal2".to_string(),
        allowed_factor_types: vec!["webauthn".to_string()],
        factor_id: None,
        metadata: serde_json::json!({"authentication": {}}),
        failed_attempts: 0,
        expires_at: Utc::now() - chrono::Duration::minutes(5),
        consumed_at: None,
    };
    set_challenge(&redis, &expired)
        .await
        .expect("expired challenge should insert");

    let active_id = replace_challenge(
        &redis,
        CreateLoginChallengeInput {
            auth_state_id,
            principal_id: None,
            tenant_id: None,
            workspace_id: None,
            purpose: "webauthn_login",
            required_level: "aal2",
            allowed_factor_types: vec!["webauthn"],
            factor_id: None,
            metadata: serde_json::json!({"authentication": {}}),
            ttl_minutes: 5,
        },
    )
    .await
    .expect("challenge should be created");

    assert_ne!(expired_id, active_id);
    assert!(
        get_challenge(&redis, expired_id)
            .await
            .expect("lookup should succeed")
            .is_none()
    );

    let active = fetch_active_challenge(
        &redis,
        active_id,
        auth_state_id,
        principal_id,
        "webauthn_login",
    )
    .await
    .expect("active challenge should be fetchable");
    assert_eq!(active.id, active_id);

    let stored = get_challenge(&redis, active_id)
        .await
        .expect("lookup should succeed")
        .expect("challenge should exist");
    assert_eq!(stored.id, active_id);
    assert_eq!(stored.auth_state_id, auth_state_id);

    let pruned = prune_expired_challenges(&redis, auth_state_id)
        .await
        .expect("prune should succeed");
    assert_eq!(pruned, 0);
}

#[cfg(test)]
#[tokio::test]
async fn failed_attempts_block_fetch_after_limit() {
    let pool = test_pool();
    let redis = crate::test_support::test_redis_pool().await;
    crate::test_support::ensure_test_database(&pool).await;
    let auth_state_id = create_state(&redis, None, "limit@example.com", "mfa", None)
        .await
        .expect("auth state should be created");
    let principal_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO principals (id, tenant_id, principal_kind, status, display_name, created_at, updated_at)
        VALUES ($1, NULL, 'human', 'active', 'Challenge Tester', $2, $2)
        "#,
    )
    .bind(principal_id)
    .bind(Utc::now())
    .execute(&pool)
    .await
    .expect("principal should be created");
    let challenge_id = replace_challenge(
        &redis,
        CreateLoginChallengeInput {
            auth_state_id,
            principal_id: Some(principal_id),
            tenant_id: None,
            workspace_id: None,
            purpose: "webauthn_login",
            required_level: "aal2",
            allowed_factor_types: vec!["webauthn"],
            factor_id: None,
            metadata: serde_json::json!({"authentication": {}}),
            ttl_minutes: 5,
        },
    )
    .await
    .expect("challenge should be created");

    for _ in 0..5 {
        let attempts = record_failed_attempt(
            &redis,
            challenge_id,
            auth_state_id,
            principal_id,
            "webauthn_login",
        )
        .await
        .expect("failed attempt should be recorded");
        assert!(attempts >= 1);
    }

    let err = fetch_active_challenge(
        &redis,
        challenge_id,
        auth_state_id,
        principal_id,
        "webauthn_login",
    )
    .await
    .expect_err("challenge should be locked");

    assert_eq!(err.code, "challenge_not_found");

    let pruned = prune_expired_challenges(&redis, auth_state_id)
        .await
        .expect("prune should succeed");
    assert_eq!(pruned, 0);
}
