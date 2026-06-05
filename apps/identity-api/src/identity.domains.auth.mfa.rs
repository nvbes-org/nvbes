use sqlx::PgPool;
use uuid::Uuid;

use super::{db::map_factor_view, types::*};
use crate::http::error::AppError;

#[path = "identity.domains.auth.mfa.recovery.rs"]
pub mod recovery;
#[path = "identity.domains.auth.mfa.totp.rs"]
pub mod totp;

pub use recovery::verify_recovery;
pub use totp::verify_totp;

pub async fn list_factors(db: &PgPool, user_id: Uuid) -> Result<MfaFactorsResult, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT id,
               factor_type::text AS factor_type,
               factor_data->>'kind' AS kind,
               status::text AS status,
               label,
               created_at,
               confirmed_at,
               last_used_at
        FROM mfa_factors
        WHERE principal_id = $1
          AND status = 'active'
        ORDER BY created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(db)
    .await?;

    Ok(MfaFactorsResult {
        mfa_enabled: !rows.is_empty(),
        factors: rows.into_iter().map(map_factor_view).collect(),
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
    sqlx::query(
        r#"
        UPDATE mfa_factors
        SET status = 'revoked', last_used_at = NOW()
        WHERE id = $1 AND principal_id = $2
        "#,
    )
    .bind(factor_id)
    .bind(user_id)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn has_active_factor(db: &PgPool, principal_id: Uuid) -> Result<bool, AppError> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM mfa_factors WHERE principal_id = $1 AND status = 'active')",
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
    if has_totp {
        methods.push("totp".to_string());
    }
    if has_webauthn {
        methods.push("webauthn".to_string());
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
        ]);

        assert_eq!(methods, vec!["totp", "webauthn", "recovery"]);
    }
}
