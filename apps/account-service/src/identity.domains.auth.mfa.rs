use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::{db::map_factor_view, types::*};
use crate::http::error::AppError;

#[path = "identity.domains.auth.mfa.recovery.rs"]
pub mod recovery;
#[path = "identity.domains.auth.mfa.totp.rs"]
pub mod totp;

pub use recovery::verify_recovery;
pub use totp::verify_totp;

pub async fn list_factors(
    db: &PgPool,
    user_id: Uuid,
    limit: Option<i64>,
    cursor: Option<String>,
) -> Result<MfaFactorsResult, AppError> {
    let limit = limit.unwrap_or(50).clamp(1, 200) as usize;
    let cursor = cursor
        .as_deref()
        .map(nvbes_core::pagination::decode_cursor::<nvbes_core::pagination::KeysetCursor>)
        .transpose()
        .map_err(|_| AppError::bad_request("invalid_cursor", "Pagination cursor is invalid."))?;
    let mfa_enabled: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM mfa_factors WHERE principal_id = $1 AND status = 'active' AND factor_type <> 'email')",
    )
    .bind(user_id)
    .fetch_one(db)
    .await?;
    let rows = sqlx::query(
        r#"
        SELECT id,
               factor_type::text AS factor_type,
               factor_data->>'kind' AS kind,
               webauthn_assurance,
               webauthn_backup_eligible,
               webauthn_backup_state,
               webauthn_sign_count,
               webauthn_attestation_format,
               status::text AS status,
               label,
               created_at,
               confirmed_at,
               last_used_at
        FROM mfa_factors
        WHERE principal_id = $1
          AND status = 'active'
          AND ($2::timestamp with time zone IS NULL OR (created_at, id) < ($2, $3))
        ORDER BY created_at DESC, id DESC
        LIMIT $4
        "#,
    )
    .bind(user_id)
    .bind(cursor.as_ref().map(|c| c.created_at))
    .bind(cursor.as_ref().map(|c| c.id))
    .bind((limit + 1) as i64)
    .fetch_all(db)
    .await?;

    let page = nvbes_core::pagination::page_from_rows(rows, limit, |row| {
        nvbes_core::pagination::KeysetCursor {
            created_at: row.get("created_at"),
            id: row.get("id"),
        }
    });
    let next_cursor = page
        .next_cursor
        .map(|c| nvbes_core::pagination::encode_cursor(&c))
        .transpose()
        .map_err(|_| {
            AppError::internal("pagination_error", "Failed to encode pagination cursor.")
        })?;
    let factors = page
        .items
        .into_iter()
        .map(map_factor_view)
        .collect::<Vec<_>>();
    Ok(MfaFactorsResult {
        mfa_enabled,
        factors,
        next_cursor,
        has_more: page.has_more,
    })
}

pub async fn get_factor(
    db: &PgPool,
    user_id: Uuid,
    factor_id: Uuid,
) -> Result<MfaFactorView, AppError> {
    let row = sqlx::query(
        r#"
        SELECT id,
               factor_type::text AS factor_type,
               factor_data->>'kind' AS kind,
               webauthn_assurance,
               webauthn_backup_eligible,
               webauthn_backup_state,
               webauthn_sign_count,
               webauthn_attestation_format,
               status::text AS status,
               label,
               created_at,
               confirmed_at,
               last_used_at
        FROM mfa_factors
        WHERE id = $1
          AND principal_id = $2
        LIMIT 1
        "#,
    )
    .bind(factor_id)
    .bind(user_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found("mfa_factor_not_found", "MFA factor not found."))?;

    Ok(map_factor_view(row))
}

pub async fn remove_factor(db: &PgPool, user_id: Uuid, factor_id: Uuid) -> Result<(), AppError> {
    let requires_privilege_check = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM mfa_factors
          WHERE id = $1
            AND principal_id = $2
            AND factor_type = 'webauthn'
            AND status = 'active'
        )
        "#,
    )
    .bind(factor_id)
    .bind(user_id)
    .fetch_one(db)
    .await?;
    let privileged = if requires_privilege_check {
        super::mfa_policy::principal_has_privileged_role(db, user_id).await?
    } else {
        false
    };

    let mut tx = db.begin().await?;
    let factor = sqlx::query(
        r#"
        SELECT factor_type::text AS factor_type, status::text AS status
        FROM mfa_factors
        WHERE id = $1 AND principal_id = $2
        FOR UPDATE
        "#,
    )
    .bind(factor_id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::not_found("mfa_factor_not_found", "MFA factor not found."))?;
    let factor_type: String = factor.get("factor_type");
    let status: String = factor.get("status");

    if status == "active" {
        let active_factors = sqlx::query(
            r#"
            SELECT id, factor_type::text AS factor_type
            FROM mfa_factors
            WHERE principal_id = $1
              AND status = 'active'
              AND factor_type <> 'email'
            FOR UPDATE
            "#,
        )
        .bind(user_id)
        .fetch_all(&mut *tx)
        .await?;
        if factor_type != "email" && active_factors.len() <= 1 {
            return Err(AppError::forbidden(
                "last_mfa_factor_cannot_be_removed",
                "Add another authentication factor before removing this one.",
            ));
        }

        if factor_type == "webauthn" {
            let remaining_passkeys = active_factors
                .iter()
                .filter(|row| {
                    row.get::<String, _>("factor_type") == "webauthn"
                        && row.get::<Uuid, _>("id") != factor_id
                })
                .count();
            if privileged && remaining_passkeys == 0 {
                return Err(AppError::forbidden(
                    "privileged_passkey_required",
                    "A privileged account must retain at least one passkey or security key.",
                ));
            }
        }
    }

    sqlx::query(
        r#"
        UPDATE mfa_factors
        SET status = 'revoked', last_used_at = NOW()
        WHERE id = $1 AND principal_id = $2
        "#,
    )
    .bind(factor_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(())
}

pub async fn has_active_factor(db: &PgPool, principal_id: Uuid) -> Result<bool, AppError> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM mfa_factors WHERE principal_id = $1 AND status = 'active' AND factor_type <> 'email')",
    )
    .bind(principal_id)
    .fetch_one(db)
    .await?;
    Ok(exists)
}

pub async fn list_login_methods(db: &PgPool, principal_id: Uuid) -> Result<Vec<String>, AppError> {
    let factor_types = sqlx::query_scalar::<_, String>(
        r#"
        SELECT DISTINCT factor_type::text AS factor_type
        FROM mfa_factors
        WHERE principal_id = $1
          AND status = 'active'
        "#,
    )
    .bind(principal_id)
    .fetch_all(db)
    .await?;

    Ok(login_methods_from_factor_types(&factor_types))
}

pub fn login_methods_from_factor_types(factor_types: &[String]) -> Vec<String> {
    let has_totp = factor_types.iter().any(|factor_type| factor_type == "totp");
    let has_webauthn = factor_types
        .iter()
        .any(|factor_type| factor_type == "webauthn");
    let has_recovery = factor_types
        .iter()
        .any(|factor_type| factor_type == "recovery_code");
    let mut methods = Vec::with_capacity(3);
    if has_webauthn {
        methods.push("webauthn".to_string());
    }
    if has_totp {
        methods.push("totp".to_string());
    }
    if has_recovery {
        methods.push("recovery".to_string());
    }
    methods
}

#[cfg(test)]
mod tests {
    use super::login_methods_from_factor_types;

    #[test]
    fn login_methods_are_ordered_by_preference() {
        let methods = login_methods_from_factor_types(&[
            "recovery_code".to_string(),
            "webauthn".to_string(),
            "totp".to_string(),
            "email".to_string(),
        ]);

        assert_eq!(methods, vec!["webauthn", "totp", "recovery"]);
    }
}

#[cfg(test)]
#[path = "identity.domains.auth.mfa.invariants.tests.rs"]
mod invariant_tests;
