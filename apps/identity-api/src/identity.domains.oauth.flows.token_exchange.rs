use sqlx::Row;
use uuid::Uuid;

use crate::app::AppState;
use crate::domains::oauth::service::TokenExchangeInput;
use crate::domains::oauth::service::TokenView;
use crate::http::error::AppError;

pub async fn token_exchange(
    state: &AppState,
    input: TokenExchangeInput,
) -> Result<TokenView, AppError> {
    let audience = input
        .audience
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("nvbes-drive-api");
    if audience != "nvbes-drive-api" {
        return Err(AppError::bad_request(
            "invalid_audience",
            "Drive delegated tokens must target the nvbes-drive-api audience.",
        ));
    }
    let client_row = sqlx::query(
        r#"
        SELECT
            id,
            client_id,
            client_secret_hash,
            client_assertion_required,
            client_type::text AS client_type,
            revoked_at
        FROM oauth_clients
        WHERE client_id = $1
        LIMIT 1
        "#,
    )
    .bind(&input.client_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| {
        AppError::unauthorized("invalid_client", "The OAuth client is not registered.")
    })?;

    if client_row
        .get::<Option<chrono::DateTime<chrono::Utc>>, _>("revoked_at")
        .is_some()
    {
        return Err(AppError::unauthorized(
            "invalid_client",
            "The OAuth client has been revoked.",
        ));
    }

    let client_uuid: Uuid = client_row.get("id");
    let client_secret_hash: String = client_row.get("client_secret_hash");
    let client_assertion_required: bool = client_row.get("client_assertion_required");
    let client_type: String = client_row.get("client_type");
    let client_display_id: String = client_row.get("client_id");

    if !crate::domains::oauth::validation::is_public_client_type(&client_type) {
        if client_assertion_required && !input.client_assertion_verified {
            return Err(AppError::unauthorized(
                "invalid_client",
                "private_key_jwt client authentication is required.",
            ));
        }

        if !input.client_assertion_verified {
            let secret = input.client_secret.as_ref().ok_or_else(|| {
                AppError::unauthorized("invalid_client", "Client authentication is required.")
            })?;
            crate::domains::oauth::verify_client_secret(secret, &client_secret_hash)?;
        }
    } else if let Some(ref secret) = input.client_secret {
        crate::domains::oauth::verify_client_secret(secret, &client_secret_hash)?;
    }

    let subject_claims =
        validate_subject_token(state, &input.subject_token, &input.subject_token_type).await?;

    validate_subject_is_user(&subject_claims)?;

    let (actor_sub, actor_client_id) = if let Some(ref actor_token) = input.actor_token {
        let actor_claims =
            validate_actor_token(state, actor_token, input.actor_token_type.as_deref()).await?;

        if actor_claims.sub != client_uuid.to_string()
            && actor_claims.client_id.as_deref() != Some(&client_display_id)
        {
            return Err(AppError::unauthorized(
                "invalid_grant",
                "The actor token does not match the authenticated client.",
            ));
        }

        (actor_claims.sub.clone(), actor_claims.client_id.clone())
    } else {
        (client_uuid.to_string(), Some(client_display_id.clone()))
    };

    let scope_str = input
        .scope
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or(&subject_claims.scope);
    let requested_scopes: Vec<&str> = scope_str
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .collect();
    let subject_scopes: Vec<&str> = subject_claims
        .scope
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .collect();
    for requested in &requested_scopes {
        if !subject_scopes.contains(requested) {
            return Err(AppError::forbidden(
                "invalid_scope",
                &format!(
                    "The requested scope '{}' exceeds the subject token scope.",
                    requested
                ),
            ));
        }
    }
    if let Some(ref actor_token) = input.actor_token {
        let actor_claims =
            validate_actor_token(state, actor_token, input.actor_token_type.as_deref()).await?;
        let actor_scopes: Vec<&str> = actor_claims
            .scope
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .collect();
        for requested in &requested_scopes {
            if !actor_scopes.contains(requested) {
                return Err(AppError::forbidden(
                    "invalid_scope",
                    &format!(
                        "The requested scope '{}' exceeds the actor token scope.",
                        requested
                    ),
                ));
            }
        }

        if actor_claims.workspace_id != subject_claims.workspace_id
            || actor_claims.organization_id != subject_claims.organization_id
            || actor_claims.tenant_id != subject_claims.tenant_id
        {
            return Err(AppError::forbidden(
                "workspace_context_mismatch",
                "The actor token must target the same tenant, organization, and workspace as the subject token.",
            ));
        }
    }

    crate::domains::oauth::policies_eval::ensure_client_policy(
        &state.db,
        &input.client_id,
        subject_claims
            .tenant_id
            .as_ref()
            .and_then(|tenant_id| Uuid::parse_str(tenant_id).ok()),
        subject_claims
            .organization_id
            .as_ref()
            .and_then(|organization_id| Uuid::parse_str(organization_id).ok()),
        subject_claims
            .workspace_id
            .as_ref()
            .and_then(|workspace_id| Uuid::parse_str(workspace_id).ok()),
        scope_str,
        Some(audience),
        &[],
    )
    .await?;

    let access_token = state.jwt.generate_token_exchange(
        &subject_claims,
        &actor_sub,
        actor_client_id.as_deref(),
        scope_str,
        Some(audience),
    )?;
    metrics::counter!("identity_oauth_token_exchange_total").increment(1);

    Ok(TokenView {
        access_token,
        token_type: "Bearer".to_string(),
        expires_in: state.jwt.access_token_expiry.num_seconds(),
        refresh_token: None,
        scope: scope_str.to_string(),
        authorization_details: subject_claims.authorization_details,
        issued_token_type: Some("urn:ietf:params:oauth:token-type:access_token".to_string()),
    })
}

async fn validate_subject_token(
    state: &AppState,
    token: &str,
    token_type: &str,
) -> Result<crate::domains::auth::jwt::types::TokenClaims, AppError> {
    if token_type != "urn:ietf:params:oauth:token-type:access_token"
        && token_type != "urn:ietf:params:oauth:token-type:refresh_token"
    {
        return Err(AppError::bad_request(
            "invalid_request",
            &format!("Unsupported subject_token_type: {}", token_type),
        ));
    }

    let expected_type = if token_type == "urn:ietf:params:oauth:token-type:refresh_token" {
        "refresh"
    } else {
        "access"
    };

    state
        .jwt
        .validate_token(Some(state), token, expected_type)
        .await
        .map_err(|_| {
            AppError::unauthorized("invalid_grant", "The subject token is invalid or expired.")
        })
}

fn validate_subject_is_user(
    claims: &crate::domains::auth::jwt::types::TokenClaims,
) -> Result<(), AppError> {
    if claims.sid == Uuid::nil().to_string() {
        return Err(AppError::unauthorized(
            "invalid_grant",
            "The subject token must represent a user, not a machine client.",
        ));
    }

    if claims.amr.contains(&"m2m".to_string()) {
        return Err(AppError::unauthorized(
            "invalid_grant",
            "The subject token must represent a user, not a machine client.",
        ));
    }

    Ok(())
}

async fn validate_actor_token(
    state: &AppState,
    token: &str,
    token_type: Option<&str>,
) -> Result<crate::domains::auth::jwt::types::TokenClaims, AppError> {
    let token_type = token_type.unwrap_or("urn:ietf:params:oauth:token-type:access_token");
    if token_type != "urn:ietf:params:oauth:token-type:access_token" {
        return Err(AppError::bad_request(
            "invalid_request",
            &format!("Unsupported actor_token_type: {}", token_type),
        ));
    }

    let claims = state.jwt.decode_token(token, "access").map_err(|_| {
        AppError::unauthorized("invalid_grant", "The actor token is invalid or expired.")
    })?;

    if !claims.amr.contains(&"m2m".to_string()) {
        return Err(AppError::unauthorized(
            "invalid_grant",
            "The actor token must be a machine-to-machine token.",
        ));
    }

    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppConfig, AppState};
    use crate::domains::oauth::service::{ClientAuthentication, TokenExchangeInput};
    use axum::{Json, extract::State, http::HeaderMap};
    use base64::Engine;
    use chrono::Utc;
    use sqlx::PgPool;

    fn test_pool() -> PgPool {
        let url = std::env::var("DATABASE_URL")
            .or_else(|_| std::env::var("NVBES_DATABASE_URL"))
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/nvbes".to_string());
        PgPool::connect_lazy(&url).expect("valid pool")
    }

    async fn test_state(pool: &PgPool) -> AppState {
        unsafe {
            std::env::set_var("NVBES_ENV", "development");
            std::env::set_var("NVBES_WORKSPACE_ROOT", "/Users/shayn/Development/nvbes");
        }

        let config = AppConfig {
            database_url: std::env::var("DATABASE_URL")
                .or_else(|_| std::env::var("NVBES_DATABASE_URL"))
                .unwrap_or_else(|_| {
                    "postgres://postgres:postgres@localhost:5432/nvbes".to_string()
                }),
            environment: "development".to_string(),
            app_name: "identity-oauth-token-exchange-test".to_string(),
            redis_url: std::env::var("NVBES_REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            redis_password: std::env::var("NVBES_REDIS_PASSWORD")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            redis_max_connections: std::env::var("NVBES_REDIS_MAX_CONNECTIONS")
                .ok()
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(10),
            ..Default::default()
        };

        crate::test_support::ensure_test_redis().await;
        crate::test_support::ensure_test_database(pool).await;

        AppState::bootstrap(&config, pool.clone())
            .await
            .expect("app state bootstrap should succeed")
    }

    async fn db_supports_current_oauth_schema(pool: &PgPool) -> bool {
        sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
              SELECT 1
              FROM information_schema.columns
              WHERE table_name = 'oauth_clients'
                AND column_name = 'client_assertion_required'
            )
            "#,
        )
        .fetch_one(pool)
        .await
        .unwrap_or(false)
    }

    async fn cleanup(pool: &PgPool, tenant_id: Uuid) {
        sqlx::query("DELETE FROM tenants WHERE id = $1")
            .bind(tenant_id)
            .execute(pool)
            .await
            .ok();
    }

    fn basic_auth_header(client_id: &str, client_secret: &str) -> String {
        let encoded = base64::engine::general_purpose::STANDARD
            .encode(format!("{client_id}:{client_secret}"));
        format!("Basic {encoded}")
    }

    async fn seed_exchange_context(pool: &PgPool) -> (Uuid, Uuid, Uuid, Uuid, String, String) {
        crate::test_support::ensure_test_redis().await;
        crate::test_support::ensure_test_database(pool).await;

        let tenant_id = Uuid::new_v4();
        let principal_id = Uuid::new_v4();
        let service_principal_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();
        let client_uuid = Uuid::new_v4();
        let client_id = format!("gxoc_exchange_{}", Uuid::new_v4().simple());
        let client_secret = format!("gxo_exchange_{}", Uuid::new_v4().simple());
        let client_secret_hash =
            crate::domains::oauth::hash_client_secret(&client_secret).expect("hash should work");
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO tenants (id, kind, name, slug, status, security_tier, created_at, updated_at)
            VALUES ($1, 'team', 'Token Exchange Tenant', $2, 'active', 'standard', $3, $3)
            "#,
        )
        .bind(tenant_id)
        .bind(format!("tenant-{}", tenant_id))
        .bind(now)
        .execute(pool)
        .await
        .expect("tenant insert should succeed");

        sqlx::query(
            r#"
            INSERT INTO workspaces (id, tenant_id, name, workspace_type, plan_code, created_at, updated_at)
            VALUES ($1, $2, 'Token Exchange Workspace', 'team', 'team_plus', $3, $3)
            "#,
        )
        .bind(workspace_id)
        .bind(tenant_id)
        .bind(now)
        .execute(pool)
        .await
        .expect("workspace insert should succeed");

        sqlx::query(
            r#"
            INSERT INTO workspace_policies (
              workspace_id,
              member_can_create_share_links,
              require_admin_approval_for_member_share,
              default_share_link_ttl_days,
              max_share_link_ttl_days,
              updated_at
            )
            VALUES ($1, false, true, 7, 30, $2)
            ON CONFLICT (workspace_id) DO NOTHING
            "#,
        )
        .bind(workspace_id)
        .bind(now)
        .execute(pool)
        .await
        .expect("workspace policy insert should succeed");

        sqlx::query(
            r#"
            INSERT INTO principals (id, tenant_id, principal_kind, status, display_name, created_at, updated_at)
            VALUES ($1, $2, 'human', 'active', 'Exchange User', $3, $3)
            "#,
        )
        .bind(principal_id)
        .bind(tenant_id)
        .bind(now)
        .execute(pool)
        .await
        .expect("user principal insert should succeed");

        sqlx::query(
            r#"
            INSERT INTO users (
              principal_id, email, firstname, lastname, username,
              email_verified_at, status, created_at, updated_at
            )
            VALUES ($1, $2, 'Exchange', 'User', $3, NOW(), 'active', $4, $4)
            "#,
        )
        .bind(principal_id)
        .bind(format!("exchange-{}@example.com", principal_id))
        .bind(format!("exchange-{}", principal_id))
        .bind(now)
        .execute(pool)
        .await
        .expect("user insert should succeed");

        sqlx::query(
            r#"
            INSERT INTO workspace_memberships (workspace_id, principal_id, role, status, source, created_at, updated_at)
            VALUES ($1, $2, 'owner', 'active', 'manual', $3, $3)
            "#,
        )
        .bind(workspace_id)
        .bind(principal_id)
        .bind(now)
        .execute(pool)
        .await
        .expect("user workspace membership insert should succeed");

        sqlx::query(
            r#"
            INSERT INTO principals (id, tenant_id, principal_kind, status, display_name, created_at, updated_at)
            VALUES ($1, $2, 'service_account', 'active', 'Exchange Robot', $3, $3)
            "#,
        )
        .bind(service_principal_id)
        .bind(tenant_id)
        .bind(now)
        .execute(pool)
        .await
        .expect("service principal insert should succeed");

        sqlx::query(
            r#"
            INSERT INTO service_accounts (
              principal_id,
              tenant_id,
              workspace_id,
              created_by_principal_id,
              name,
              description,
              auth_method,
              client_id,
              last_rotated_at,
              created_at,
              updated_at
            )
            VALUES ($1, $2, $3, $4, 'Exchange Robot', 'delegation actor', 'oauth_client_credentials', $5, $6, $6, $6)
            "#,
        )
        .bind(service_principal_id)
        .bind(tenant_id)
        .bind(workspace_id)
        .bind(principal_id)
        .bind(&client_id)
        .bind(now)
        .execute(pool)
        .await
        .expect("service account insert should succeed");

        sqlx::query(
            r#"
            INSERT INTO workspace_memberships (workspace_id, principal_id, role, status, source, created_at, updated_at)
            VALUES ($1, $2, 'viewer', 'active', 'system', $3, $3)
            "#,
        )
        .bind(workspace_id)
        .bind(service_principal_id)
        .bind(now)
        .execute(pool)
        .await
        .expect("service workspace membership insert should succeed");

        sqlx::query(
            r#"
            INSERT INTO oauth_clients (
              id,
              client_id,
              client_secret_hash,
              name,
              redirect_uris,
              tenant_id,
              owner_scope_type,
              owner_scope_id,
              client_type,
              revoked_at,
              created_at,
              updated_at
            )
            VALUES ($1, $2, $3, 'Exchange Client', ARRAY['https://example.com/callback'], $4, 'workspace', $5, 'service', NULL, $6, $6)
            "#,
        )
        .bind(client_uuid)
        .bind(&client_id)
        .bind(&client_secret_hash)
        .bind(tenant_id)
        .bind(workspace_id)
        .bind(now)
        .execute(pool)
        .await
        .expect("oauth client insert should succeed");

        sqlx::query(
            r#"
            INSERT INTO oauth_client_policies (
              client_id,
              scope_type,
              scope_id,
              allowed_scopes,
              allowed_audiences,
              allowed_resources,
              required_acr,
              status,
              created_at
            )
            VALUES (
              $1,
              'workspace',
              $2,
              ARRAY['drive.files.read', 'drive.workspace.read'],
              ARRAY['nvbes-drive-api'],
              ARRAY['drive'],
              'aal1',
              'active',
              $3
            )
            "#,
        )
        .bind(client_uuid)
        .bind(workspace_id)
        .bind(now)
        .execute(pool)
        .await
        .expect("oauth policy insert should succeed");

        (
            tenant_id,
            principal_id,
            service_principal_id,
            workspace_id,
            client_id,
            client_secret,
        )
    }

    #[tokio::test]
    async fn token_exchange_preserves_user_subject_and_sets_machine_actor() {
        let pool = test_pool();
        if !db_supports_current_oauth_schema(&pool).await {
            eprintln!("skipping test: local database is missing recent oauth schema migrations");
            return;
        }

        let state = test_state(&pool).await;
        let (
            tenant_id,
            user_principal_id,
            service_principal_id,
            workspace_id,
            client_id,
            client_secret,
        ) = seed_exchange_context(&pool).await;

        let session_id = Uuid::new_v4();
        let subject_pair = state
            .jwt
            .generate_token_pair_with_session(
                user_principal_id,
                Some(workspace_id),
                None,
                "drive.files.read drive.workspace.read",
                Some(session_id),
                Some(tenant_id),
                None,
                Some("aal1"),
                Some(vec!["pwd".to_string()]),
                None,
                Some(Utc::now().timestamp()),
                None,
            )
            .expect("subject token should be created");

        let actor = crate::domains::oauth::flows::client_credentials_grant(
            &state,
            ClientAuthentication {
                client_id: client_id.clone(),
                client_secret: Some(client_secret.clone()),
                client_assertion: None,
                client_assertion_verified: false,
            },
            Some("drive.files.read"),
            None,
        )
        .await
        .expect("actor token should be created");

        let exchanged = token_exchange(
            &state,
            TokenExchangeInput {
                subject_token: subject_pair.access_token,
                subject_token_type: "urn:ietf:params:oauth:token-type:access_token".to_string(),
                actor_token: Some(actor.access_token),
                actor_token_type: Some("urn:ietf:params:oauth:token-type:access_token".to_string()),
                client_id: client_id.clone(),
                client_secret: Some(client_secret),
                client_assertion_verified: false,
                scope: Some("drive.files.read".to_string()),
                audience: None,
                resource: None,
                requested_token_type: Some(
                    "urn:ietf:params:oauth:token-type:access_token".to_string(),
                ),
            },
        )
        .await
        .expect("token exchange should succeed");

        let exchanged_claims = state
            .jwt
            .decode_token(&exchanged.access_token, "access")
            .expect("exchanged token should decode");

        let expected_subject = user_principal_id.to_string();
        let expected_workspace = workspace_id.to_string();
        let expected_tenant = tenant_id.to_string();
        let expected_actor = service_principal_id.to_string();

        assert_eq!(exchanged.scope, "drive.files.read");
        assert_eq!(exchanged_claims.sub, expected_subject);
        assert_eq!(
            exchanged_claims.workspace_id.as_deref(),
            Some(expected_workspace.as_str())
        );
        assert_eq!(
            exchanged_claims.tenant_id.as_deref(),
            Some(expected_tenant.as_str())
        );
        assert_eq!(
            exchanged_claims
                .act
                .as_ref()
                .map(|actor| actor.sub.as_str()),
            Some(expected_actor.as_str())
        );
        assert_eq!(
            exchanged_claims
                .act
                .as_ref()
                .and_then(|actor| actor.client_id.as_deref()),
            Some(client_id.as_str())
        );

        cleanup(&pool, tenant_id).await;
    }

    #[tokio::test]
    async fn http_token_exchange_preserves_user_subject_and_sets_machine_actor() {
        let pool = test_pool();
        if !db_supports_current_oauth_schema(&pool).await {
            eprintln!("skipping test: local database is missing recent oauth schema migrations");
            return;
        }

        let state = test_state(&pool).await;
        let (
            tenant_id,
            user_principal_id,
            service_principal_id,
            workspace_id,
            client_id,
            client_secret,
        ) = seed_exchange_context(&pool).await;
        let session_id = Uuid::new_v4();
        let subject_pair = state
            .jwt
            .generate_token_pair_with_session(
                user_principal_id,
                Some(workspace_id),
                None,
                "drive.files.read drive.workspace.read",
                Some(session_id),
                Some(tenant_id),
                None,
                Some("aal1"),
                Some(vec!["pwd".to_string()]),
                None,
                Some(Utc::now().timestamp()),
                None,
            )
            .expect("subject token should be created");

        let actor_response = crate::domains::oauth::routes::token::token(
            State(state.clone()),
            basic_headers(&client_id, &client_secret),
            axum::Form(crate::domains::oauth::routes::token::TokenRequest {
                grant_type: "client_credentials".to_string(),
                code: None,
                refresh_token: None,
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                code_verifier: None,
                device_code: None,
                scope: Some("drive.files.read".to_string()),
                audience: Some("nvbes-drive-api".to_string()),
                subject_token: None,
                subject_token_type: None,
                actor_token: None,
                actor_token_type: None,
                requested_token_type: None,
                client_assertion_type: None,
                client_assertion: None,
            }),
        )
        .await
        .expect("actor token endpoint should respond");

        let actor_body =
            serde_json::to_value(actor_response.0).expect("actor token should serialize");
        let actor_token = actor_body["access_token"]
            .as_str()
            .expect("actor token should be present")
            .to_string();

        let exchange_response = crate::domains::oauth::routes::token::token(
            State(state.clone()),
            basic_headers(&client_id, &client_secret),
            axum::Form(crate::domains::oauth::routes::token::TokenRequest {
                grant_type: "urn:ietf:params:oauth:grant-type:token-exchange".to_string(),
                code: None,
                refresh_token: None,
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                code_verifier: None,
                device_code: None,
                scope: Some("drive.files.read".to_string()),
                audience: Some("nvbes-drive-api".to_string()),
                subject_token: Some(subject_pair.access_token),
                subject_token_type: Some(
                    "urn:ietf:params:oauth:token-type:access_token".to_string(),
                ),
                actor_token: Some(actor_token),
                actor_token_type: Some("urn:ietf:params:oauth:token-type:access_token".to_string()),
                requested_token_type: Some(
                    "urn:ietf:params:oauth:token-type:access_token".to_string(),
                ),
                client_assertion_type: None,
                client_assertion: None,
            }),
        )
        .await
        .expect("token exchange endpoint should respond");

        let exchange_body =
            serde_json::to_value(exchange_response.0).expect("exchange token should serialize");
        let exchanged_token = exchange_body["access_token"]
            .as_str()
            .expect("exchanged token should be present");

        let exchanged_claims = state
            .jwt
            .decode_token(exchanged_token, "access")
            .expect("exchanged token should decode");

        assert_eq!(exchanged_claims.sub, user_principal_id.to_string());
        assert_eq!(
            exchanged_claims.workspace_id.as_deref(),
            Some(workspace_id.to_string().as_str())
        );
        assert_eq!(
            exchanged_claims.tenant_id.as_deref(),
            Some(tenant_id.to_string().as_str())
        );
        assert_eq!(
            exchanged_claims
                .act
                .as_ref()
                .map(|actor| actor.sub.as_str()),
            Some(service_principal_id.to_string().as_str())
        );
        assert_eq!(
            exchanged_claims
                .act
                .as_ref()
                .and_then(|actor| actor.client_id.as_deref()),
            Some(client_id.as_str())
        );

        cleanup(&pool, tenant_id).await;
    }

    fn basic_headers(client_id: &str, client_secret: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            basic_auth_header(client_id, client_secret)
                .parse()
                .expect("authorization header should parse"),
        );
        headers
    }
}
