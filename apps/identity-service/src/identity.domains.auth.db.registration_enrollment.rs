use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::http::error::AppError;

pub async fn insert_tx(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
    token_hash: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO registration_enrollment_tokens (
          token_hash,
          principal_id,
          expires_at
        )
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(token_hash)
    .bind(principal_id)
    .bind(expires_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn insert(
    db: &PgPool,
    principal_id: Uuid,
    token_hash: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO registration_enrollment_tokens (
          token_hash,
          principal_id,
          expires_at
        )
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(token_hash)
    .bind(principal_id)
    .bind(expires_at)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn consume_tx(
    tx: &mut Transaction<'_, Postgres>,
    token_hash: &str,
) -> Result<Uuid, AppError> {
    sqlx::query_scalar(
        r#"
        UPDATE registration_enrollment_tokens
        SET consumed_at = NOW()
        WHERE token_hash = $1
          AND consumed_at IS NULL
          AND expires_at > NOW()
        RETURNING principal_id
        "#,
    )
    .bind(token_hash)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| {
        AppError::unauthorized(
            "registration_enrollment_invalid",
            "Registration enrollment has expired or is invalid.",
        )
    })
}

pub async fn consume_all_for_principal_tx(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE registration_enrollment_tokens
        SET consumed_at = COALESCE(consumed_at, NOW())
        WHERE principal_id = $1
          AND consumed_at IS NULL
        "#,
    )
    .bind(principal_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn delete_all_for_principal_tx(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM registration_enrollment_tokens WHERE principal_id = $1")
        .bind(principal_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    use super::{consume_tx, insert};

    #[tokio::test]
    async fn enrollment_token_can_only_be_consumed_once() {
        let db = crate::test_support::shared_test_pool();
        crate::test_support::ensure_test_database(&db).await;

        let tenant_id = Uuid::new_v4();
        let principal_id = Uuid::new_v4();
        let token_hash = format!("enrollment-{principal_id}");
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO tenants (
              id, kind, name, slug, status, security_tier, created_at, updated_at
            )
            VALUES ($1, 'personal', 'Enrollment Test', $2, 'active', 'standard', $3, $3)
            "#,
        )
        .bind(tenant_id)
        .bind(format!("enrollment-{tenant_id}"))
        .bind(now)
        .execute(&db)
        .await
        .expect("tenant should be inserted");

        sqlx::query(
            r#"
            INSERT INTO principals (
              id, tenant_id, principal_kind, status, display_name, created_at, updated_at
            )
            VALUES ($1, $2, 'human', 'active', 'Enrollment Test', $3, $3)
            "#,
        )
        .bind(principal_id)
        .bind(tenant_id)
        .bind(now)
        .execute(&db)
        .await
        .expect("principal should be inserted");

        insert(&db, principal_id, &token_hash, now + Duration::minutes(5))
            .await
            .expect("enrollment should be inserted");

        let mut first_tx = db.begin().await.expect("transaction should start");
        assert_eq!(
            consume_tx(&mut first_tx, &token_hash)
                .await
                .expect("first consumption should succeed"),
            principal_id
        );
        first_tx.commit().await.expect("transaction should commit");

        let mut replay_tx = db.begin().await.expect("transaction should start");
        let replay_error = consume_tx(&mut replay_tx, &token_hash)
            .await
            .expect_err("replay must be rejected");
        assert_eq!(replay_error.code, "registration_enrollment_invalid");
        replay_tx
            .rollback()
            .await
            .expect("transaction should roll back");

        let expired_hash = format!("expired-{principal_id}");
        sqlx::query(
            r#"
            INSERT INTO registration_enrollment_tokens (
              token_hash, principal_id, created_at, expires_at
            )
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(&expired_hash)
        .bind(principal_id)
        .bind(now - Duration::minutes(2))
        .bind(now - Duration::minutes(1))
        .execute(&db)
        .await
        .expect("expired enrollment should be inserted");

        let mut expired_tx = db.begin().await.expect("transaction should start");
        let expired_error = consume_tx(&mut expired_tx, &expired_hash)
            .await
            .expect_err("expired enrollment must be rejected");
        assert_eq!(expired_error.code, "registration_enrollment_invalid");
        expired_tx
            .rollback()
            .await
            .expect("transaction should roll back");

        sqlx::query("DELETE FROM tenants WHERE id = $1")
            .bind(tenant_id)
            .execute(&db)
            .await
            .expect("test tenant should be deleted");
    }
}
