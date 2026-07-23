use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::domains::auth::{normalize_email, types::EmailAddressView, validate_email};
use crate::http::error::AppError;

#[path = "identity.domains.auth.db.emails.map.rs"]
mod map;
#[path = "identity.domains.auth.db.emails.policy.rs"]
mod policy;

pub(crate) use map::email_constraint_error;
pub use policy::{
    active_email_recipients, primary_email_policy_hours, verified_mfa_eligible_emails,
};

pub async fn list_email_addresses(
    db: &PgPool,
    principal_id: Uuid,
    cursor: Option<&nvbes_core::pagination::KeysetCursor>,
    limit: i64,
) -> Result<Vec<EmailAddressView>, AppError> {
    let rows = sqlx::query(
        "SELECT id, principal_id, email, is_primary, verified_at, created_at, updated_at FROM user_email_addresses WHERE principal_id = $1 AND deleted_at IS NULL AND ($2::timestamp with time zone IS NULL OR (created_at, id) < ($2, $3)) ORDER BY created_at DESC, id DESC LIMIT $4",
    )
    .bind(principal_id)
    .bind(cursor.map(|c| c.created_at))
    .bind(cursor.map(|c| c.id))
    .bind(limit)
    .fetch_all(db).await?;

    Ok(rows.into_iter().map(map::email_address_view).collect())
}

pub async fn insert_secondary_email(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
    email: &str,
) -> Result<EmailAddressView, AppError> {
    let normalized_email = normalize_email(email);
    validate_email(&normalized_email)?;

    let row = sqlx::query(
        r#"
        INSERT INTO user_email_addresses (
          principal_id,
          email,
          normalized_email,
          is_primary,
          verified_at,
          created_at,
          updated_at
        )
        VALUES ($1, $2, $3, FALSE, NULL, NOW(), NOW())
        RETURNING id, principal_id, email, is_primary, verified_at, created_at, updated_at
        "#,
    )
    .bind(principal_id)
    .bind(&normalized_email)
    .bind(&normalized_email)
    .fetch_one(&mut **tx)
    .await
    .map_err(map::email_constraint_error)?;

    Ok(map::email_address_view(row))
}

pub async fn fetch_email_address_for_update(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
    email_address_id: Uuid,
) -> Result<EmailAddressView, AppError> {
    let row = sqlx::query(
        r#"
        SELECT id, principal_id, email, is_primary, verified_at, created_at, updated_at
        FROM user_email_addresses
        WHERE principal_id = $1
          AND id = $2
          AND deleted_at IS NULL
        FOR UPDATE
        "#,
    )
    .bind(principal_id)
    .bind(email_address_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| AppError::not_found("email_address_not_found", "Email address not found."))?;

    Ok(map::email_address_view(row))
}

pub async fn mark_secondary_verified(
    db: &PgPool,
    principal_id: Uuid,
    email_address_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE user_email_addresses
        SET verified_at = COALESCE(verified_at, NOW()),
            updated_at = NOW()
        WHERE principal_id = $1
          AND id = $2
          AND is_primary = FALSE
          AND deleted_at IS NULL
        "#,
    )
    .bind(principal_id)
    .bind(email_address_id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn promote_secondary_email(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
    email_address_id: Uuid,
    min_age_hours: i32,
) -> Result<EmailAddressView, AppError> {
    let candidate = fetch_email_address_for_update(tx, principal_id, email_address_id).await?;
    if candidate.is_primary {
        return Err(AppError::bad_request(
            "email_already_primary",
            "This email is already the primary email.",
        ));
    }
    if candidate.verified_at.is_none() {
        return Err(AppError::forbidden(
            "email_not_verified",
            "This email must be verified before it can become primary.",
        ));
    }

    let min_age_hours = min_age_hours.max(0);
    let eligible = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT created_at <= NOW() - make_interval(hours => $3::int)
        FROM user_email_addresses
        WHERE principal_id = $1 AND id = $2
        "#,
    )
    .bind(principal_id)
    .bind(email_address_id)
    .bind(min_age_hours)
    .fetch_one(&mut **tx)
    .await?;

    if !eligible {
        return Err(AppError::forbidden(
            "email_too_new_for_primary",
            "This email cannot become primary yet.",
        ));
    }

    let current_primary_email: String =
        sqlx::query_scalar("SELECT email FROM users WHERE principal_id = $1 FOR UPDATE")
            .bind(principal_id)
            .fetch_one(&mut **tx)
            .await?;

    sqlx::query(
        r#"
        UPDATE user_email_addresses
        SET is_primary = FALSE, updated_at = NOW()
        WHERE principal_id = $1
          AND is_primary = TRUE
          AND deleted_at IS NULL
        "#,
    )
    .bind(principal_id)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE user_email_addresses
        SET is_primary = TRUE, updated_at = NOW()
        WHERE principal_id = $1 AND id = $2
        "#,
    )
    .bind(principal_id)
    .bind(email_address_id)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE users
        SET email = $2,
            email_verified_at = NOW(),
            status = 'active',
            updated_at = NOW()
        WHERE principal_id = $1
        "#,
    )
    .bind(principal_id)
    .bind(&candidate.email)
    .execute(&mut **tx)
    .await?;

    demote_previous_primary(tx, principal_id, &current_primary_email).await?;
    fetch_email_address_for_update(tx, principal_id, email_address_id).await
}

async fn demote_previous_primary(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
    current_primary_email: &str,
) -> Result<(), AppError> {
    let normalized = normalize_email(current_primary_email);
    sqlx::query(
        r#"
        INSERT INTO user_email_addresses (
          principal_id,
          email,
          normalized_email,
          is_primary,
          verified_at,
          created_at,
          updated_at
        )
        VALUES ($1, $2, $3, FALSE, NOW(), NOW(), NOW())
        ON CONFLICT (normalized_email) WHERE deleted_at IS NULL
        DO UPDATE SET is_primary = FALSE,
                      verified_at = COALESCE(user_email_addresses.verified_at, NOW()),
                      updated_at = NOW()
        "#,
    )
    .bind(principal_id)
    .bind(&normalized)
    .bind(&normalized)
    .execute(&mut **tx)
    .await
    .map_err(map::email_constraint_error)?;
    Ok(())
}

pub async fn delete_secondary_email(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
    email_address_id: Uuid,
) -> Result<(), AppError> {
    let address = fetch_email_address_for_update(tx, principal_id, email_address_id).await?;
    if address.is_primary {
        return Err(AppError::forbidden(
            "primary_email_cannot_be_deleted",
            "The primary email cannot be deleted.",
        ));
    }

    sqlx::query(
        r#"
        UPDATE user_email_addresses
        SET deleted_at = NOW(),
            updated_at = NOW(),
            is_primary = FALSE
        WHERE principal_id = $1 AND id = $2
        "#,
    )
    .bind(principal_id)
    .bind(email_address_id)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE mfa_factors
        SET status = 'revoked'
        WHERE principal_id = $1
          AND factor_type = 'email'
          AND status = 'active'
          AND factor_data->>'email_id' = $2
        "#,
    )
    .bind(principal_id)
    .bind(email_address_id.to_string())
    .execute(&mut **tx)
    .await?;
    Ok(())
}
