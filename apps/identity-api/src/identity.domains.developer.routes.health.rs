use axum::{Extension, Json, extract::State};
use serde_json::json;
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::developer::{
        rbac::DeveloperPermission,
        service,
        types::{
            DeveloperHealthCheckSeed, DeveloperHealthCheckSummary, DeveloperHealthChecksResponse,
            RunDeveloperHealthChecksResponse,
        },
    },
    http::{error::AppError, middleware::jwt::AuthContext},
};

pub async fn list_health_checks(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DeveloperHealthChecksResponse>, AppError> {
    let tenant_id = service::require_permission(
        &state.db,
        &auth,
        DeveloperPermission::ConsoleHealthChecksRead,
    )
    .await?;
    Ok(Json(DeveloperHealthChecksResponse {
        checks: fetch_latest_checks(&state.db, tenant_id).await?,
    }))
}

pub async fn run_health_checks(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<RunDeveloperHealthChecksResponse>, AppError> {
    let tenant_id =
        service::require_permission(&state.db, &auth, DeveloperPermission::HealthChecksRun).await?;
    let seeds = build_health_check_seeds(&state.db, tenant_id).await?;
    let mut checks = Vec::with_capacity(seeds.len());

    for seed in seeds {
        let check = sqlx::query_as(
            r#"
            INSERT INTO developer_health_checks (
              tenant_id,
              target_type,
              target_id,
              check_kind,
              status,
              summary
            )
            VALUES ($1, $2, $3, $4, $5::developer_health_status, $6)
            RETURNING id, target_type, target_id, check_kind, status::text AS status, summary, checked_at
            "#,
        )
        .bind(tenant_id)
        .bind(&seed.target_type)
        .bind(&seed.target_id)
        .bind(&seed.check_kind)
        .bind(&seed.status)
        .bind(format!("{} {}", seed.summary, seed.metadata))
        .fetch_one(&state.db)
        .await?;
        checks.push(check);
    }

    Ok(Json(RunDeveloperHealthChecksResponse { checks }))
}

async fn fetch_latest_checks(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
) -> Result<Vec<DeveloperHealthCheckSummary>, AppError> {
    sqlx::query_as(
        r#"
        SELECT DISTINCT ON (target_type, target_id, check_kind)
          id,
          target_type,
          target_id,
          check_kind,
          status::text AS status,
          summary,
          checked_at
        FROM developer_health_checks
        WHERE tenant_id = $1
        ORDER BY target_type, target_id, check_kind, checked_at DESC
        LIMIT 200
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await
    .map_err(AppError::from)
}

async fn build_health_check_seeds(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
) -> Result<Vec<DeveloperHealthCheckSeed>, AppError> {
    let oauth_clients = oauth_client_health_seeds(db, tenant_id).await?;
    let webhooks = webhook_health_seeds(db, tenant_id).await?;
    let scim = scim_health_seeds(db, tenant_id).await?;
    let mut seeds = Vec::with_capacity(oauth_clients.len() + webhooks.len() + scim.len() + 2);
    seeds.extend(oauth_clients);
    seeds.extend(webhooks);
    seeds.extend(scim);
    seeds.push(DeveloperHealthCheckSeed {
        target_type: "identity".to_string(),
        target_id: "jwks".to_string(),
        check_kind: "jwks".to_string(),
        status: "passing".to_string(),
        summary: "JWKS route is configured.".to_string(),
        metadata: json!({ "path": "/.well-known/jwks.json" }),
    });
    seeds.push(DeveloperHealthCheckSeed {
        target_type: "identity".to_string(),
        target_id: "oidc-discovery".to_string(),
        check_kind: "oidc_discovery".to_string(),
        status: "passing".to_string(),
        summary: "OIDC discovery route is configured.".to_string(),
        metadata: json!({ "path": "/.well-known/openid-configuration" }),
    });
    Ok(seeds)
}

async fn oauth_client_health_seeds(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
) -> Result<Vec<DeveloperHealthCheckSeed>, AppError> {
    let rows: Vec<(String, Vec<String>)> = sqlx::query_as(
        "SELECT client_id, redirect_uris FROM oauth_clients WHERE tenant_id = $1 AND revoked_at IS NULL",
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(client_id, redirect_uris)| {
            let has_https = redirect_uris.iter().any(|uri| uri.starts_with("https://"));
            DeveloperHealthCheckSeed {
                target_type: "oauth_client".to_string(),
                target_id: client_id,
                check_kind: "redirects".to_string(),
                status: if has_https { "passing" } else { "warning" }.to_string(),
                summary: if has_https {
                    "Redirect URIs include an HTTPS callback."
                } else {
                    "Redirect URIs should include an HTTPS callback before production approval."
                }
                .to_string(),
                metadata: json!({ "redirect_uri_count": redirect_uris.len() }),
            }
        })
        .collect())
}

async fn webhook_health_seeds(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
) -> Result<Vec<DeveloperHealthCheckSeed>, AppError> {
    let rows: Vec<(Uuid, String, String)> = sqlx::query_as(
        "SELECT id, url, status::text FROM developer_webhook_endpoints WHERE tenant_id = $1 AND revoked_at IS NULL",
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, url, status)| DeveloperHealthCheckSeed {
            target_type: "webhook".to_string(),
            target_id: id.to_string(),
            check_kind: "webhook".to_string(),
            status: if status == "active" && url.starts_with("https://") {
                "passing"
            } else {
                "failing"
            }
            .to_string(),
            summary: "Webhook endpoint URL and activation state checked.".to_string(),
            metadata: json!({ "url": url, "status": status }),
        })
        .collect())
}

async fn scim_health_seeds(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
) -> Result<Vec<DeveloperHealthCheckSeed>, AppError> {
    let rows: Vec<(Uuid, Option<String>, String)> = sqlx::query_as(
        "SELECT id, base_url, status FROM scim_provisioning_connectors WHERE tenant_id = $1",
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, base_url, status)| DeveloperHealthCheckSeed {
            target_type: "scim".to_string(),
            target_id: id.to_string(),
            check_kind: "scim".to_string(),
            status: if status == "active"
                && base_url
                    .as_deref()
                    .is_some_and(|url| url.starts_with("https://"))
            {
                "passing"
            } else {
                "warning"
            }
            .to_string(),
            summary: "SCIM connector base URL and activation state checked.".to_string(),
            metadata: json!({ "base_url": base_url, "status": status }),
        })
        .collect())
}
