use sqlx::Row;
use uuid::Uuid;

use crate::http::error::AppError;

pub trait OAuthManagementAuth {
    fn user_id(&self) -> Uuid;
    fn tenant_id(&self) -> Option<Uuid>;
    fn workspace_id(&self) -> Option<Uuid>;
}

impl OAuthManagementAuth for crate::http::middleware::jwt::AuthContext {
    fn user_id(&self) -> Uuid {
        self.user_id
    }

    fn tenant_id(&self) -> Option<Uuid> {
        self.tenant_id
    }

    fn workspace_id(&self) -> Option<Uuid> {
        self.workspace_id
    }
}

impl OAuthManagementAuth for crate::domains::auth::types::AuthContext {
    fn user_id(&self) -> Uuid {
        self.user_id
    }

    fn tenant_id(&self) -> Option<Uuid> {
        self.tenant_id
    }

    fn workspace_id(&self) -> Option<Uuid> {
        self.workspace_id
    }
}

pub fn hash_client_secret(secret: &str) -> Result<String, AppError> {
    use argon2::{Algorithm, Argon2, Params, PasswordHasher, Version, password_hash::SaltString};
    use password_hash::rand_core::OsRng;
    let salt = SaltString::generate(&mut OsRng);
    let params = Params::new(65536, 3, 4, None)
        .map_err(|e| AppError::internal("client_secret_hash_failed", format!("{}", e)))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    argon2
        .hash_password(secret.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| AppError::internal("client_secret_hash_failed", format!("{}", e)))
}

pub fn verify_client_secret(secret: &str, hash_value: &str) -> Result<(), AppError> {
    use argon2::{Argon2, PasswordVerifier, password_hash::PasswordHash};
    let parsed_hash = PasswordHash::new(hash_value)
        .map_err(|e| AppError::internal("client_secret_hash_invalid", format!("{}", e)))?;
    Argon2::default()
        .verify_password(secret.as_bytes(), &parsed_hash)
        .map_err(|_| AppError::unauthorized("invalid_client", "The client secret is invalid."))
}

pub async fn verify_client_secret_with_overlap(
    db: &sqlx::PgPool,
    client_id: &str,
    secret: &str,
    main_hash: &str,
) -> Result<(), AppError> {
    if verify_client_secret(secret, main_hash).is_ok() {
        return Ok(());
    }

    let valid_hashes: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT client_secret_hash
        FROM developer_client_secret_versions
        WHERE client_id = $1
          AND revoked_at IS NULL
          AND (
            status = 'active'
            OR (status = 'overlap' AND (expires_at IS NULL OR expires_at > NOW()))
          )
        "#,
    )
    .bind(client_id)
    .fetch_all(db)
    .await
    .map_err(AppError::from)?;

    for hash in valid_hashes {
        if hash != main_hash && verify_client_secret(secret, &hash).is_ok() {
            return Ok(());
        }
    }

    Err(AppError::unauthorized("invalid_client", "The client secret is invalid."))
}


pub fn generate_user_code() -> String {
    use rand::Rng;
    let chars = b"BCDFGHJKLMNPQRSTVWXZ";
    let mut rng = rand::rng();
    let part1: String = (0..4)
        .map(|_| chars[rng.random_range(0..chars.len())] as char)
        .collect();
    let part2: String = (0..4)
        .map(|_| chars[rng.random_range(0..chars.len())] as char)
        .collect();
    format!("{}-{}", part1, part2)
}

pub fn resolve_management_scope(
    auth: &impl OAuthManagementAuth,
) -> Result<(String, Uuid), AppError> {
    if let Some(workspace_id) = auth.workspace_id() {
        return Ok(("workspace".to_string(), workspace_id));
    }

    if let Some(tenant_id) = auth.tenant_id() {
        return Ok(("tenant".to_string(), tenant_id));
    }

    Err(AppError::forbidden(
        "scope_unavailable",
        "A tenant or workspace context is required.",
    ))
}

pub fn resolve_owner_scope(
    auth: &impl OAuthManagementAuth,
    requested_scope_type: Option<&str>,
    requested_scope_id: Option<Uuid>,
) -> Result<(String, Uuid), AppError> {
    let default_scope = resolve_management_scope(auth)?;
    match (requested_scope_type, requested_scope_id) {
        (Some(scope_type), Some(scope_id)) => {
            let scope_type = scope_type.trim().to_lowercase();
            if scope_type == default_scope.0 && scope_id == default_scope.1 {
                Ok((scope_type, scope_id))
            } else {
                Err(AppError::forbidden(
                    "scope_forbidden",
                    "The requested scope is not allowed from the current context.",
                ))
            }
        }
        (None, None) => Ok(default_scope),
        _ => Err(AppError::bad_request(
            "validation_failed",
            "scope_type and scope_id must be provided together.",
        )),
    }
}

pub fn resolve_policy_scope(
    auth: &impl OAuthManagementAuth,
    requested_scope_type: Option<&str>,
    requested_scope_id: Option<Uuid>,
) -> Result<(String, Uuid), AppError> {
    let default_scope = resolve_management_scope(auth)?;
    match (requested_scope_type, requested_scope_id) {
        (Some(scope_type), Some(scope_id)) => {
            let scope_type = scope_type.trim().to_lowercase();
            if scope_type == "workspace"
                && auth
                    .workspace_id()
                    .is_some_and(|current| current == scope_id)
            {
                return Ok((scope_type, scope_id));
            }
            if scope_type == "tenant" && auth.tenant_id().is_some_and(|current| current == scope_id)
            {
                return Ok((scope_type, scope_id));
            }
            Err(AppError::forbidden(
                "scope_forbidden",
                "The requested policy scope is not allowed from the current context.",
            ))
        }
        (None, None) => Ok(default_scope),
        _ => Err(AppError::bad_request(
            "validation_failed",
            "scope_type and scope_id must be provided together.",
        )),
    }
}

pub async fn client_uuid_by_client_id(
    db: &sqlx::PgPool,
    tenant_id: Option<Uuid>,
    client_id: &str,
) -> Result<Uuid, AppError> {
    let row = sqlx::query(
        r#"
        SELECT id
        FROM oauth_clients
        WHERE client_id = $1
          AND ($2::uuid IS NULL OR tenant_id = $2)
          AND revoked_at IS NULL
        LIMIT 1
        "#,
    )
    .bind(client_id)
    .bind(tenant_id)
    .fetch_optional(db)
    .await?;

    let row = row.ok_or_else(|| {
        AppError::not_found("client_not_found", "The OAuth client was not found.")
    })?;

    Ok(row.get("id"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::oauth::validation::{
        validate_redirect_uri_allowed, validate_redirect_uri_match,
    };

    #[test]
    fn validate_redirect_uri_allowed_accepts_registered_uri() {
        let allowed = vec![
            "https://example.com/callback".to_string(),
            "https://example.com/alt".to_string(),
        ];

        assert!(validate_redirect_uri_allowed(&allowed, "https://example.com/alt").is_ok());
    }

    #[test]
    fn validate_redirect_uri_allowed_rejects_unregistered_uri() {
        let allowed = vec!["https://example.com/callback".to_string()];
        let err = validate_redirect_uri_allowed(&allowed, "https://attacker.example/callback")
            .expect_err("expected redirect URI validation to fail");

        assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
        assert_eq!(err.code, "invalid_redirect_uri");
        assert_eq!(
            err.message,
            "The redirect URI is not registered for this OAuth client."
        );
    }

    #[test]
    fn validate_redirect_uri_match_rejects_mismatch() {
        let err = validate_redirect_uri_match(
            "https://example.com/callback",
            Some("https://example.com/other"),
        )
        .expect_err("expected redirect URI mismatch to fail");

        assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
        assert_eq!(err.code, "invalid_grant");
        assert_eq!(
            err.message,
            "The redirect URI does not match the authorization request."
        );
    }

    #[test]
    fn test_client_secret_hashing_owasp_params() {
        let secret = "client-secret-key-12345-extremely-secure";
        let hash = hash_client_secret(secret).expect("hashing should succeed");

        assert!(hash.contains("$argon2id$"));
        assert!(hash.contains("m=65536,t=3,p=4"));

        verify_client_secret(secret, &hash).expect("verification should run");

        let not_ok = verify_client_secret("wrong-secret", &hash);
        assert!(not_ok.is_err());
    }
}
