use sqlx::Row;
use uuid::Uuid;

use crate::app::AppState;
use crate::http::error::AppError;

use super::{ClientAuthentication, TokenView};

pub async fn client_credentials_grant(
    state: &AppState,
    client_auth: ClientAuthentication,
    scope: Option<&str>,
    audience: Option<&str>,
) -> Result<TokenView, AppError> {
    let audience = audience
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("nvbes-drive-api");
    if audience != "nvbes-drive-api" {
        return Err(AppError::bad_request(
            "invalid_audience",
            "Drive machine tokens must target the nvbes-drive-api audience.",
        ));
    }
    let client = sqlx::query(
        r#"
        SELECT
            oauth_clients.id,
            oauth_clients.client_id,
            oauth_clients.client_secret_hash,
            oauth_clients.client_assertion_required,
            oauth_clients.client_type::text AS client_type,
            oauth_clients.owner_scope_type::text AS owner_scope_type,
            oauth_clients.owner_scope_id,
            sa.principal_id AS service_account_principal_id,
            sa.workspace_id AS service_account_workspace_id,
            w.tenant_id,
            w.organization_id,
            w.data_region::text AS workspace_region,
            wm.role::text AS service_account_role,
            p.status::text AS principal_status
        FROM oauth_clients
        LEFT JOIN service_accounts sa ON sa.client_id = oauth_clients.client_id
        LEFT JOIN workspaces w ON w.id = sa.workspace_id
        LEFT JOIN workspace_memberships wm
          ON wm.workspace_id = sa.workspace_id
         AND wm.principal_id = sa.principal_id
         AND wm.status = 'active'
        LEFT JOIN principals p ON p.id = sa.principal_id
        WHERE oauth_clients.client_id = $1
          AND oauth_clients.revoked_at IS NULL
        LIMIT 1
        "#,
    )
    .bind(&client_auth.client_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| {
        AppError::unauthorized(
            "invalid_client",
            "The client is not registered or has been revoked.",
        )
    })?;

    let client_uuid: Uuid = client.get("id");
    let client_secret_hash: String = client.get("client_secret_hash");
    let client_assertion_required: bool = client.get("client_assertion_required");
    let client_type: String = client.get("client_type");
    let owner_scope_type: String = client.get("owner_scope_type");
    let owner_scope_id: Uuid = client.get("owner_scope_id");
    let service_account_principal_id: Option<Uuid> = client.get("service_account_principal_id");
    let service_account_workspace_id: Option<Uuid> = client.get("service_account_workspace_id");
    let tenant_id: Option<Uuid> = client.get("tenant_id");
    let organization_id: Option<Uuid> = client.get("organization_id");
    let workspace_region: Option<String> = client.get("workspace_region");
    let service_account_role: Option<String> = client.get("service_account_role");
    let principal_status: Option<String> = client.get("principal_status");

    if crate::domains::oauth::validation::is_public_client_type(&client_type) {
        return Err(AppError::unauthorized(
            "unauthorized_client",
            "Public OAuth clients cannot use client_credentials.",
        ));
    }

    if client_assertion_required && !client_auth.client_assertion_verified {
        return Err(AppError::unauthorized(
            "invalid_client",
            "private_key_jwt client authentication is required.",
        ));
    }

    if !client_auth.client_assertion_verified {
        let client_secret = client_auth.client_secret.as_deref().ok_or_else(|| {
            AppError::unauthorized(
                "invalid_client",
                "Client authentication is required for client_credentials grant.",
            )
        })?;
        crate::domains::oauth::verify_client_secret(client_secret, &client_secret_hash)?;
    }

    let scope_str = scope.unwrap_or("");
    if !scope_str.is_empty() {
        let policy = sqlx::query(
            r#"
            SELECT
                allowed_scopes,
                allowed_audiences,
                status::text AS status
            FROM oauth_client_policies
            WHERE client_id = $1
              AND scope_type = $2::scope_type
              AND scope_id = $3
            LIMIT 1
            "#,
        )
        .bind(client_uuid)
        .bind(&owner_scope_type)
        .bind(owner_scope_id)
        .fetch_optional(&state.db)
        .await?;

        if let Some(policy) = policy {
            let status: String = policy.get("status");
            if status != "active" {
                return Err(AppError::forbidden(
                    "client_not_allowed",
                    "The OAuth client policy is not active.",
                ));
            }

            let allowed_scopes: Vec<String> = policy.get("allowed_scopes");
            let allowed_scopes = crate::domains::oauth::normalize_scopes(allowed_scopes);
            let allowed_audiences = crate::domains::oauth::normalize_resources(
                policy.get::<Vec<String>, _>("allowed_audiences"),
            );
            let requested_scopes = crate::domains::oauth::normalize_scopes(
                scope_str
                    .split_whitespace()
                    .map(ToOwned::to_owned)
                    .collect(),
            );
            if !requested_scopes
                .iter()
                .all(|s| allowed_scopes.iter().any(|a| a == s))
            {
                return Err(AppError::forbidden(
                    "client_scope_not_allowed",
                    "The requested scope is not approved for this OAuth client.",
                ));
            }
            if !allowed_audiences.is_empty()
                && !allowed_audiences.iter().any(|allowed| allowed == audience)
            {
                return Err(AppError::forbidden(
                    "invalid_scope",
                    "The requested audience is not approved for this OAuth client.",
                ));
            }
        }
    }

    let service_account_principal_id = service_account_principal_id.ok_or_else(|| {
        AppError::forbidden(
            "service_account_required",
            "This OAuth client is not attached to a workspace service account.",
        )
    })?;
    let workspace_id = service_account_workspace_id.ok_or_else(|| {
        AppError::forbidden(
            "workspace_scope_required",
            "This OAuth client is not attached to a workspace service account.",
        )
    })?;
    let tenant_id = tenant_id.ok_or_else(|| {
        AppError::forbidden(
            "tenant_context_required",
            "This OAuth client is not attached to a tenant workspace.",
        )
    })?;
    if principal_status.as_deref() != Some("active") || service_account_role.is_none() {
        return Err(AppError::forbidden(
            "service_account_inactive",
            "The OAuth client service account is inactive or unassigned.",
        ));
    }
    if owner_scope_type != "workspace" || owner_scope_id != workspace_id {
        return Err(AppError::forbidden(
            "workspace_scope_required",
            "Machine-to-machine Drive clients must be scoped to a workspace.",
        ));
    }

    let access_token = state.jwt.generate_m2m_access_token(
        &client_auth.client_id,
        service_account_principal_id,
        tenant_id,
        organization_id,
        workspace_id,
        workspace_region,
        scope_str,
        Some(audience),
    )?;
    metrics::counter!("identity_oauth_client_credentials_total").increment(1);

    Ok(TokenView {
        access_token,
        token_type: "Bearer".to_string(),
        expires_in: state.jwt.access_token_expiry.num_seconds(),
        refresh_token: None,
        scope: scope_str.to_string(),
        authorization_details: Vec::new(),
        issued_token_type: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppConfig, AppState};
    use crate::domains::oauth::service::ClientAuthentication;
    use axum::{Form, Json, extract::State, http::HeaderMap};
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
            app_name: "identity-oauth-client-credentials-test".to_string(),
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

    async fn cleanup(pool: &PgPool, tenant_id: Uuid) {
        sqlx::query("DELETE FROM tenants WHERE id = $1")
            .bind(tenant_id)
            .execute(pool)
            .await
            .ok();
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

    fn basic_auth_header(client_id: &str, client_secret: &str) -> String {
        let encoded = base64::engine::general_purpose::STANDARD
            .encode(format!("{client_id}:{client_secret}"));
        format!("Basic {encoded}")
    }

    async fn seed_service_client(pool: &PgPool) -> (Uuid, String, String, Uuid, Uuid, Uuid) {
        crate::test_support::ensure_test_redis().await;
        crate::test_support::ensure_test_database(pool).await;

        let tenant_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();
        let principal_id = Uuid::new_v4();
        let client_uuid = Uuid::new_v4();
        let client_id = format!("gxoc_test_{}", Uuid::new_v4().simple());
        let client_secret = format!("gxo_test_{}", Uuid::new_v4().simple());
        let client_secret_hash =
            crate::domains::oauth::hash_client_secret(&client_secret).expect("hash should work");
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO tenants (id, kind, name, slug, status, security_tier, created_at, updated_at)
            VALUES ($1, 'team', 'OAuth Test Tenant', $2, 'active', 'standard', $3, $3)
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
            VALUES ($1, $2, 'OAuth Test Workspace', 'team', 'team_plus', $3, $3)
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
            VALUES ($1, $2, 'service_account', 'active', 'Drive CI Robot', $3, $3)
            "#,
        )
        .bind(principal_id)
        .bind(tenant_id)
        .bind(now)
        .execute(pool)
        .await
        .expect("principal insert should succeed");

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
              secret_hash,
              public_key_jwk,
              last_rotated_at,
              created_at,
              updated_at
            )
            VALUES ($1, $2, $3, NULL, 'Drive CI Robot', 'test service account', 'oauth_client_credentials', $4, NULL, NULL, $5, $5, $5)
            "#,
        )
        .bind(principal_id)
        .bind(tenant_id)
        .bind(workspace_id)
        .bind(&client_id)
        .bind(now)
        .execute(pool)
        .await
        .expect("service account insert should succeed");

        sqlx::query(
            r#"
            INSERT INTO workspace_memberships (workspace_id, principal_id, role, status, source, created_at, updated_at)
            VALUES ($1, $2, 'member', 'active', 'system', $3, $3)
            "#,
        )
        .bind(workspace_id)
        .bind(principal_id)
        .bind(now)
        .execute(pool)
        .await
        .expect("workspace membership insert should succeed");

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
            VALUES ($1, $2, $3, 'Drive API client', ARRAY['https://example.com/callback'], $4, 'workspace', $5, 'service', NULL, $6, $6)
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
            client_id,
            client_secret,
            principal_id,
            workspace_id,
            client_uuid,
        )
    }

    #[tokio::test]
    async fn client_credentials_token_introspection_exposes_service_account_context() {
        let pool = test_pool();
        if !db_supports_current_oauth_schema(&pool).await {
            eprintln!("skipping test: local database is missing recent oauth schema migrations");
            return;
        }
        let state = test_state(&pool).await;
        let (tenant_id, client_id, client_secret, principal_id, workspace_id, _) =
            seed_service_client(&pool).await;

        let token = client_credentials_grant(
            &state,
            ClientAuthentication {
                client_id: client_id.clone(),
                client_secret: Some(client_secret.clone()),
                client_assertion: None,
                client_assertion_verified: false,
            },
            Some("drive.files.read drive.workspace.read"),
            None,
        )
        .await
        .expect("client_credentials should succeed");

        let introspection = crate::domains::oauth::flows::introspect_token(
            &state,
            &client_id,
            &token.access_token,
            Some("access_token".to_string()),
            None,
        )
        .await
        .expect("introspection should succeed");

        assert!(introspection.active);
        assert_eq!(
            introspection.principal_type.as_deref(),
            Some("service_account")
        );
        let expected_subject = principal_id.to_string();
        assert_eq!(
            introspection.sub.as_deref(),
            Some(expected_subject.as_str())
        );
        assert_eq!(introspection.workspace_id, Some(workspace_id));
        assert_eq!(introspection.tenant_id, Some(tenant_id));
        assert_eq!(introspection.role.as_deref(), Some("member"));
        assert_eq!(introspection.client_id.as_deref(), Some(client_id.as_str()));
        assert!(introspection.amr.iter().any(|method| method == "m2m"));

        cleanup(&pool, tenant_id).await;
    }

    #[tokio::test]
    async fn http_client_credentials_and_introspection_expose_service_account_context() {
        let pool = test_pool();
        if !db_supports_current_oauth_schema(&pool).await {
            eprintln!("skipping test: local database is missing recent oauth schema migrations");
            return;
        }

        let state = test_state(&pool).await;
        let (tenant_id, client_id, client_secret, principal_id, workspace_id, _) =
            seed_service_client(&pool).await;
        let token_response = crate::domains::oauth::routes::token::token(
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
                scope: Some("drive.files.read drive.workspace.read".to_string()),
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
        .expect("token endpoint should respond");

        let token_body =
            serde_json::to_value(token_response.0).expect("token body should serialize");
        let access_token = token_body["access_token"]
            .as_str()
            .expect("token response should include access_token");

        let introspection_response = crate::domains::oauth::routes::introspect::introspect(
            State(state),
            basic_headers(&client_id, &client_secret),
            Json(
                crate::domains::oauth::routes::introspect::IntrospectRequest {
                    token: access_token.to_string(),
                    token_type_hint: Some("access_token".to_string()),
                },
            ),
        )
        .await
        .expect("introspection endpoint should respond");

        let introspection =
            serde_json::to_value(introspection_response.0).expect("introspection should serialize");

        assert_eq!(introspection["active"], true);
        assert_eq!(introspection["principal_type"], "service_account");
        assert_eq!(introspection["sub"], principal_id.to_string());
        assert_eq!(introspection["workspace_id"], workspace_id.to_string());
        assert_eq!(introspection["tenant_id"], tenant_id.to_string());
        assert_eq!(introspection["role"], "member");
        assert_eq!(introspection["client_id"], client_id);
        assert_eq!(introspection["amr"], serde_json::json!(["m2m"]));

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
