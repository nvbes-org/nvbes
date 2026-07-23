use crate::cloud_boundary::workspace_port;
use crate::domains::auth::jwt::JwtService;
use crate::domains::auth::types::derive_display_name;
use crate::domains::oauth::flows::IntrospectionResponse;
use crate::domains::oauth::flows::tokens::{
    introspect_actor::resolve_actor_context, introspect_network::resolve_network_valid,
};
use crate::http::error::AppError;
use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::postgres::PgPool;
use uuid::Uuid;

pub async fn introspect_token(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    jwt: &JwtService,
    client_id: &str,
    token: &str,
    token_type_hint: Option<String>,
    client_ip: Option<String>,
) -> Result<IntrospectionResponse, AppError> {
    let expected_type = match token_type_hint.as_deref().map(str::trim) {
        Some("refresh_token") => "refresh",
        Some("access_token") | None => "access",
        Some(other) => {
            return Err(AppError::bad_request(
                "validation_failed",
                format!("Unsupported token_type_hint: {other}.").as_str(),
            ));
        }
    };

    let claims = match jwt.validate_token(Some(redis), token, expected_type).await {
        Ok(claims) => claims,
        Err(_) => return Ok(IntrospectionResponse::inactive()),
    };
    if claims.amr.iter().any(|method| method == "m2m")
        && !machine_token_client_is_active(db, claims.client_id.as_deref()).await?
    {
        return Ok(IntrospectionResponse::inactive());
    }

    crate::domains::oauth::policies_eval::ensure_client_policy(
        db,
        client_id,
        claims
            .tenant_id
            .as_ref()
            .and_then(|tenant_id| Uuid::parse_str(tenant_id).ok()),
        claims
            .organization_id
            .as_ref()
            .and_then(|organization_id| Uuid::parse_str(organization_id).ok()),
        claims
            .workspace_id
            .as_ref()
            .and_then(|workspace_id| Uuid::parse_str(workspace_id).ok()),
        &claims.scope,
        Some(&claims.aud),
        &[],
    )
    .await?;

    let subject_id = match Uuid::parse_str(&claims.sub) {
        Ok(value) => value,
        Err(_) => return Ok(IntrospectionResponse::inactive()),
    };
    let workspace_id = claims
        .workspace_id
        .as_ref()
        .and_then(|workspace_id| Uuid::parse_str(workspace_id).ok());
    let tenant_id = claims
        .tenant_id
        .as_ref()
        .and_then(|tenant_id| Uuid::parse_str(tenant_id).ok());
    let organization_id = claims
        .organization_id
        .as_ref()
        .and_then(|organization_id| Uuid::parse_str(organization_id).ok());

    let principal_type = if claims.amr.iter().any(|method| method == "m2m") {
        "service_account"
    } else {
        "user"
    };

    let (email, username, display_name, email_verified, acr, amr, auth_time, role) =
        if principal_type == "service_account" {
            let service_account = sqlx::query(
                r#"
                SELECT
                  sa.name,
                  sa.workspace_id,
                  p.status::text AS principal_status
                FROM service_accounts sa
                INNER JOIN principals p ON p.id = sa.principal_id
                WHERE sa.principal_id = $1
                LIMIT 1
                "#,
            )
            .bind(subject_id)
            .fetch_optional(db)
            .await?;

            let Some(service_account) = service_account else {
                return Ok(IntrospectionResponse::inactive());
            };
            if service_account.get::<String, _>("principal_status") != "active" {
                return Ok(IntrospectionResponse::inactive());
            }
            let service_account_workspace_id: Option<Uuid> = service_account.get("workspace_id");
            if service_account_workspace_id != workspace_id {
                return Ok(IntrospectionResponse::inactive());
            }
            let Some(workspace_id) = workspace_id else {
                return Ok(IntrospectionResponse::inactive());
            };
            let role = workspace_port::list_workspace_members(tenant_id, workspace_id, subject_id)
                .await?
                .into_iter()
                .find(|member| member.principal_id == subject_id && member.active)
                .map(|member| member.role);
            if role.is_none() {
                return Ok(IntrospectionResponse::inactive());
            }

            (
                None,
                None,
                service_account.get::<String, _>("name"),
                None,
                None,
                claims.amr.clone(),
                claims.auth_time,
                role,
            )
        } else {
            let user_row = sqlx::query(
                r#"
                SELECT
                  email,
                  firstname,
                  lastname,
                  username,
                  email_verified_at,
                  status::text AS status
                FROM users
                WHERE principal_id = $1
                LIMIT 1
                "#,
            )
            .bind(subject_id)
            .fetch_optional(db)
            .await?;

            let Some(user_row) = user_row else {
                return Ok(IntrospectionResponse::inactive());
            };
            if user_row.get::<String, _>("status") != "active" {
                return Ok(IntrospectionResponse::inactive());
            }

            let display_name = derive_display_name(
                user_row.get::<Option<String>, _>("firstname").as_deref(),
                user_row.get::<Option<String>, _>("lastname").as_deref(),
                user_row.get::<Option<String>, _>("username").as_deref(),
            );
            let session_id = match Uuid::parse_str(&claims.sid) {
                Ok(value) => value,
                Err(_) => return Ok(IntrospectionResponse::inactive()),
            };
            let assurance = crate::domains::oauth::assurance::resolve_assurance_context(
                db,
                redis,
                subject_id,
                Some(session_id),
                claims.client_id.as_deref(),
                tenant_id,
                organization_id,
                workspace_id,
            )
            .await?;
            if !assurance.sufficient {
                return Ok(IntrospectionResponse::inactive());
            }

            let role = if let Some(workspace_id) = workspace_id {
                workspace_port::list_workspace_members(tenant_id, workspace_id, subject_id)
                    .await?
                    .into_iter()
                    .find(|member| member.principal_id == subject_id && member.active)
                    .map(|member| member.role)
            } else {
                None
            };

            (
                Some(user_row.get("email")),
                user_row.get::<Option<String>, _>("username"),
                display_name,
                Some(
                    user_row
                        .get::<Option<DateTime<Utc>>, _>("email_verified_at")
                        .is_some(),
                ),
                Some(assurance.acr),
                assurance.amr,
                Some(assurance.auth_time),
                role,
            )
        };

    let (
        actor_principal_type,
        actor_role,
        actor_workspace_id,
        actor_organization_id,
        actor_tenant_id,
    ) = resolve_actor_context(db, claims.act.as_ref()).await?;

    let network_valid = resolve_network_valid(db, tenant_id, client_ip.as_deref()).await?;

    Ok(IntrospectionResponse {
        active: true,
        scope: Some(claims.scope),
        authorization_details: claims.authorization_details,
        client_id: claims.client_id,
        principal_type: Some(principal_type.to_string()),
        token_type: Some(expected_type.to_string()),
        sub: Some(claims.sub),
        role,
        tenant_id,
        organization_id,
        workspace_id,
        username,
        email,
        email_verified,
        display_name: Some(display_name),
        acr,
        amr,
        auth_time,
        jti: Some(claims.jti),
        sid: Some(claims.sid),
        exp: Some(claims.exp as i64),
        iat: Some(claims.iat as i64),
        nbf: Some(claims.nbf as i64),
        act: claims
            .act
            .as_ref()
            .and_then(|act| serde_json::to_value(act).ok()),
        actor_principal_type,
        actor_role,
        actor_workspace_id,
        actor_organization_id,
        actor_tenant_id,
        network_valid,
    })
}

async fn machine_token_client_is_active(
    db: &PgPool,
    token_client_id: Option<&str>,
) -> Result<bool, AppError> {
    let Some(token_client_id) = token_client_id else {
        return Ok(false);
    };

    let row = sqlx::query(
        r#"
        SELECT revoked_at
        FROM oauth_clients
        WHERE client_id = $1
        LIMIT 1
        "#,
    )
    .bind(token_client_id)
    .fetch_optional(db)
    .await?;

    let Some(row) = row else {
        return Ok(false);
    };

    Ok(row
        .get::<Option<chrono::DateTime<chrono::Utc>>, _>("revoked_at")
        .is_none())
}
