use sqlx::Row;
use uuid::Uuid;

use crate::http::error::AppError;
use sqlx::PgPool;

use super::service::PolicyEvaluation;
use super::{normalize_resources, normalize_scopes};

/// Ensure a client policy is met.
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
    let client = sqlx::query(
        r#"
        SELECT id
        FROM oauth_clients
        WHERE client_id = $1
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

    let client_uuid: Uuid = client.get("id");
    let scope = normalize_scopes(
        requested_scope
            .split_whitespace()
            .map(ToOwned::to_owned)
            .collect(),
    );

    if client_id == "drive-web" || client_id == "drive-worker" {
        return Ok(PolicyEvaluation {
            status: "active".to_string(),
            normalized_scope: scope,
        });
    }

    let policy = sqlx::query(
        r#"
        SELECT
          scope_type::text AS scope_type,
          scope_id,
          allowed_scopes,
          allowed_audiences,
          allowed_resources,
          required_acr::text AS required_acr,
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
        metrics::counter!(
            "identity_oauth_policy_denied_total",
            &[("reason", "client_approval_required")]
        )
        .increment(1);
        return Err(AppError::forbidden(
            "client_approval_required",
            "This OAuth client requires approval from a workspace administrator.",
        ));
    };

    let status: String = policy.get("status");
    if status == "blocked" {
        metrics::counter!(
            "identity_oauth_policy_denied_total",
            &[("reason", "client_blocked")]
        )
        .increment(1);
        return Err(AppError::forbidden(
            "client_blocked",
            "This OAuth client has been blocked for this scope.",
        ));
    }

    if status == "pending_approval" {
        metrics::counter!(
            "identity_oauth_policy_denied_total",
            &[("reason", "client_approval_pending")]
        )
        .increment(1);
        return Err(AppError::forbidden(
            "client_approval_pending",
            "The approval for this OAuth client is currently pending.",
        ));
    }

    let allowed_scopes = normalize_scopes(policy.get::<Vec<String>, _>("allowed_scopes"));
    if scope.is_empty()
        || !scope
            .iter()
            .all(|requested| allowed_scopes.iter().any(|allowed| allowed == requested))
    {
        metrics::counter!(
            "identity_oauth_policy_denied_total",
            &[("reason", "client_scope_not_allowed")]
        )
        .increment(1);
        return Err(AppError::forbidden(
            "client_scope_not_allowed",
            "The requested scope is not approved for this client.",
        ));
    }

    let requested_resources = normalize_resources(resource_indicators.to_vec());
    let allowed_audiences = normalize_resources(policy.get::<Vec<String>, _>("allowed_audiences"));
    let allowed_resources = normalize_resources(policy.get::<Vec<String>, _>("allowed_resources"));
    if let Some(audience) = audience.filter(|value| !value.trim().is_empty()) {
        if !allowed_audiences.is_empty()
            && !allowed_audiences.iter().any(|allowed| allowed == audience)
        {
            metrics::counter!(
                "identity_oauth_policy_denied_total",
                &[("reason", "client_audience_not_allowed")]
            )
            .increment(1);
            return Err(AppError::forbidden(
                "client_audience_not_allowed",
                "The requested audience is not approved for this client.",
            ));
        }
    }
    if !requested_resources.is_empty()
        && !allowed_resources.is_empty()
        && !requested_resources
            .iter()
            .all(|requested| allowed_resources.iter().any(|allowed| allowed == requested))
    {
        metrics::counter!(
            "identity_oauth_policy_denied_total",
            &[("reason", "client_resource_not_allowed")]
        )
        .increment(1);
        return Err(AppError::forbidden(
            "client_resource_not_allowed",
            "The requested resource indicator is not approved for this client.",
        ));
    }

    Ok(PolicyEvaluation {
        status,
        normalized_scope: scope,
    })
}
