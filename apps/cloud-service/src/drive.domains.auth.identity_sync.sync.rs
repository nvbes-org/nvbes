use chrono::Utc;

use crate::{
    domains::auth::{
        identity::IdentityIntrospectionResponse,
        identity_sync::claims::{
            IdentityUserRecord, identity_email, identity_name, is_unavailable_status,
            parse_principal_kind, required_claim_string,
        },
        types::AuthPrincipalKind,
    },
    http::error::AppError,
};

pub(crate) async fn sync_identity_user(
    db: &sqlx::PgPool,
    claims: &IdentityIntrospectionResponse,
) -> Result<IdentityUserRecord, AppError> {
    let subject = required_claim_string(claims.sub.as_deref())?;
    let principal_kind = parse_principal_kind(claims.principal_type.as_deref());
    let email = identity_email(claims, &subject, principal_kind)?;
    let name = identity_name(claims, &email, principal_kind);
    let verified = if principal_kind == AuthPrincipalKind::ServiceAccount {
        true
    } else {
        claims.email_verified.unwrap_or(false)
    };
    let desired_email_verified_at = if verified { Some(Utc::now()) } else { None };
    let desired_status = if verified || principal_kind == AuthPrincipalKind::ServiceAccount {
        "active"
    } else {
        "pending_verification"
    };

    let mut tx = db.begin().await?;

    if let Some(user) = sqlx::query_as::<_, IdentityUserRecord>(
        r#"
        SELECT id, email_verified_at, status::text AS status, identity_subject
        FROM users
        WHERE identity_subject = $1
        LIMIT 1
        "#,
    )
    .bind(&subject)
    .fetch_optional(&mut *tx)
    .await?
    {
        if is_unavailable_status(&user.status) {
            return Err(AppError::forbidden(
                "account_unavailable",
                "This account is not available.",
            ));
        }

        let user = sqlx::query_as::<_, IdentityUserRecord>(
            r#"
            UPDATE users
            SET email = $2,
                display_name = $3,
                email_verified_at = CASE
                    WHEN $4::boolean THEN COALESCE(email_verified_at, NOW())
                    ELSE email_verified_at
                END,
                status = CASE
                    WHEN $4::boolean THEN 'active'::user_status
                    ELSE status
                END,
                identity_subject = COALESCE(identity_subject, $5),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, email_verified_at, status::text AS status, identity_subject
            "#,
        )
        .bind(user.id)
        .bind(&email)
        .bind(&name)
        .bind(verified)
        .bind(&subject)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        return Ok(user);
    }

    if let Some(user) = sqlx::query_as::<_, IdentityUserRecord>(
        r#"
        SELECT id, email_verified_at, status::text AS status, identity_subject
        FROM users
        WHERE email = $1
        LIMIT 1
        "#,
    )
    .bind(&email)
    .fetch_optional(&mut *tx)
    .await?
    {
        if is_unavailable_status(&user.status) {
            return Err(AppError::forbidden(
                "account_unavailable",
                "This account is not available.",
            ));
        }

        if user
            .identity_subject
            .as_deref()
            .is_some_and(|value| value != subject)
        {
            return Err(AppError::forbidden(
                "identity_account_mismatch",
                "This account is already linked to a different Identity subject.",
            ));
        }

        let user = sqlx::query_as::<_, IdentityUserRecord>(
            r#"
            UPDATE users
            SET email = $2,
                display_name = $3,
                email_verified_at = CASE
                    WHEN $4::boolean THEN COALESCE(email_verified_at, NOW())
                    ELSE email_verified_at
                END,
                status = CASE
                    WHEN $4::boolean THEN 'active'::user_status
                    ELSE status
                END,
                identity_subject = COALESCE(identity_subject, $5),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, email_verified_at, status::text AS status, identity_subject
            "#,
        )
        .bind(user.id)
        .bind(&email)
        .bind(&name)
        .bind(verified)
        .bind(&subject)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        return Ok(user);
    }

    let user = sqlx::query_as::<_, IdentityUserRecord>(
        r#"
        INSERT INTO users (
          email,
          display_name,
          status,
          email_verified_at,
          identity_subject
        )
        VALUES ($1, $2, $3::user_status, $4, $5)
        RETURNING id, email_verified_at, status::text AS status, identity_subject
        "#,
    )
    .bind(&email)
    .bind(&name)
    .bind(desired_status)
    .bind(desired_email_verified_at)
    .bind(&subject)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(user)
}
