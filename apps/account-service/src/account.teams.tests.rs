#![allow(unused_imports)]
use sqlx::PgPool;
use uuid::Uuid;

use super::{create_and_join_for_synthetic, hash_join_code, join_team};
use crate::profile;

#[test]
fn hash_join_code_is_stable_sha256() {
    let first = hash_join_code("team_0123456789abcdef0123456789abcdef");
    let second = hash_join_code("team_0123456789abcdef0123456789abcdef");
    let other = hash_join_code("team_ffffffffffffffffffffffffffffffff");
    assert_eq!(first, second);
    assert_ne!(first, other);
    assert_eq!(first.len(), 32);
}

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn create_and_join_synthetic_assigns_member_role(pool: PgPool) {
    let owner = Uuid::new_v4();
    let member = Uuid::new_v4();
    let (team_id, role) = create_and_join_for_synthetic(&pool, owner, member)
        .await
        .unwrap();
    assert_eq!(role, "member");
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM account_team_memberships WHERE team_id=$1")
            .bind(team_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 2);
}

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn join_team_rejects_malformed_codes(pool: PgPool) {
    let principal = Uuid::new_v4();
    profile::ensure_profile(&pool, principal).await.unwrap();
    assert!(matches!(
        join_team(&pool, principal, "bad", Uuid::new_v4()).await,
        Err(crate::error::AccountError::NotFound)
    ));
    assert!(matches!(
        join_team(&pool, principal, "team_short", Uuid::new_v4()).await,
        Err(crate::error::AccountError::NotFound)
    ));
}
