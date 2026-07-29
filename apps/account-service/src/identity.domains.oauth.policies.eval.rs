use sqlx::Row;
use uuid::Uuid;

use crate::http::error::AppError;
use sqlx::PgPool;

use super::service::PolicyEvaluation;
use super::system_clients::is_system_client;
use super::{normalize_resources, normalize_scopes};

struct ClientPolicyRecord {
    status: String,
    allowed_scopes: Vec<String>,
    allowed_audiences: Vec<String>,
    allowed_resources: Vec<String>,
}

/// Ensure a client policy is met.
#[expect(
    clippy::too_many_arguments,
    reason = "Client policy evaluation keeps scope resolution inputs explicit."
)]
pub async fn ensure_client_policy(
    db: &PgPool,
    client_id: &str,
    tenant_id: Option<Uuid>,
    organization_id: Option<Uuid>,
    workspace_id: Option<Uuid>,
    requested_scope: &str,
    audience: Option<&str>,
    resource_indicators: &[String],
) -> Result<PolicyEvaluation, AppError> {
    let client_uuid = resolve_client_uuid(db, client_id).await?;
    let scope = normalize_scopes(
        requested_scope
            .split_whitespace()
            .map(ToOwned::to_owned)
            .collect(),
    );

    if is_system_client(client_id) {
        return Ok(PolicyEvaluation {
            status: "active".to_string(),
            normalized_scope: scope,
        });
    }

    let policy =
        fetch_client_policy(db, client_uuid, tenant_id, organization_id, workspace_id).await?;
    ensure_policy_status_allowed(&policy.status)?;

    let allowed_scopes = normalize_scopes(policy.allowed_scopes);
    if scope.is_empty()
        || !scope
            .iter()
            .all(|requested| allowed_scopes.iter().any(|allowed| allowed == requested))
    {
        return Err(policy_denied(
            "client_scope_not_allowed",
            "The requested scope is not approved for this client.",
        ));
    }

    let requested_resources = normalize_resources(resource_indicators.to_vec());
    let allowed_audiences = normalize_resources(policy.allowed_audiences);
    let allowed_resources = normalize_resources(policy.allowed_resources);
    if let Some(audience) = audience.filter(|value| !value.trim().is_empty())
        && !allowed_audiences.is_empty()
        && !allowed_audiences.iter().any(|allowed| allowed == audience)
    {
        return Err(policy_denied(
            "client_audience_not_allowed",
            "The requested audience is not approved for this client.",
        ));
    }
    if !requested_resources.is_empty()
        && !allowed_resources.is_empty()
        && !requested_resources
            .iter()
            .all(|requested| allowed_resources.iter().any(|allowed| allowed == requested))
    {
        return Err(policy_denied(
            "client_resource_not_allowed",
            "The requested resource indicator is not approved for this client.",
        ));
    }

    Ok(PolicyEvaluation {
        status: policy.status,
        normalized_scope: scope,
    })
}

async fn resolve_client_uuid(db: &PgPool, client_id: &str) -> Result<Uuid, AppError> {
    let client = sqlx::query(
        r#"
        SELECT id
        FROM oauth_clients
        WHERE client_id = $1
          AND revoked_at IS NULL
        LIMIT 1
        "#,
    )
    .bind(client_id)
    .fetch_optional(db)
    .await?;

    let Some(client) = client else {
        return Err(AppError::bad_request(
            "client_not_found",
            "The OAuth client is not registered.",
        ));
    };

    Ok(client.get("id"))
}

async fn fetch_client_policy(
    db: &PgPool,
    client_uuid: Uuid,
    tenant_id: Option<Uuid>,
    organization_id: Option<Uuid>,
    workspace_id: Option<Uuid>,
) -> Result<ClientPolicyRecord, AppError> {
    let policy = sqlx::query(
        r#"
        SELECT
          allowed_scopes,
          allowed_audiences,
          allowed_resources,
          status::text AS status
        FROM oauth_client_policies
        WHERE client_id = $1
          AND (
            ($2::uuid IS NOT NULL AND scope_type = 'workspace' AND scope_id = $2)
            OR ($3::uuid IS NOT NULL AND scope_type = 'organization' AND scope_id = $3)
            OR ($4::uuid IS NOT NULL AND scope_type = 'tenant' AND scope_id = $4)
          )
        ORDER BY
          CASE scope_type
            WHEN 'workspace' THEN 0
            WHEN 'organization' THEN 1
            WHEN 'tenant' THEN 2
          END
        LIMIT 1
        "#,
    )
    .bind(client_uuid)
    .bind(workspace_id)
    .bind(organization_id)
    .bind(tenant_id)
    .fetch_optional(db)
    .await?;

    let Some(policy) = policy else {
        return Err(policy_denied(
            "client_approval_required",
            "This OAuth client requires approval from a workspace administrator.",
        ));
    };

    Ok(ClientPolicyRecord {
        status: policy.get("status"),
        allowed_scopes: policy.get("allowed_scopes"),
        allowed_audiences: policy.get("allowed_audiences"),
        allowed_resources: policy.get("allowed_resources"),
    })
}

fn ensure_policy_status_allowed(status: &str) -> Result<(), AppError> {
    match status {
        "blocked" => Err(policy_denied(
            "client_blocked",
            "This OAuth client has been blocked for this scope.",
        )),
        "pending_approval" => Err(policy_denied(
            "client_approval_pending",
            "The approval for this OAuth client is currently pending.",
        )),
        _ => Ok(()),
    }
}

fn policy_denied(code: &'static str, message: &'static str) -> AppError {
    metrics::counter!("identity_oauth_policy_denied_total", &[("reason", code)]).increment(1);
    AppError::forbidden(code, message)
}
