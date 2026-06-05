use crate::app::AppState;
use crate::domains::auth::types::derive_display_name;
use crate::http::error::AppError;
use chrono::{DateTime, Utc};
use sqlx::Row;
use uuid::Uuid;

use super::{IntrospectionResponse, TokenView};

/// Introspect a token.
pub async fn introspect_token(
    state: &AppState,
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

    let claims = match state
        .jwt
        .validate_token(Some(state), token, expected_type)
        .await
    {
        Ok(claims) => claims,
        Err(_) => return Ok(IntrospectionResponse::inactive()),
    };

    crate::domains::oauth::policies_eval::ensure_client_policy(
        &state.db,
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
                  wm.role::text AS role,
                  p.status::text AS principal_status
                FROM service_accounts sa
                INNER JOIN principals p ON p.id = sa.principal_id
                LEFT JOIN workspace_memberships wm
                  ON wm.workspace_id = sa.workspace_id
                 AND wm.principal_id = sa.principal_id
                 AND wm.status = 'active'
                WHERE sa.principal_id = $1
                LIMIT 1
                "#,
            )
            .bind(subject_id)
            .fetch_optional(&state.db)
            .await?;

            let Some(service_account) = service_account else {
                return Ok(IntrospectionResponse::inactive());
            };
            if service_account.get::<String, _>("principal_status") != "active" {
                return Ok(IntrospectionResponse::inactive());
            }
            let role: Option<String> = service_account.get("role");
            if role.is_none()
                || service_account.get::<Option<Uuid>, _>("workspace_id") != workspace_id
            {
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
            .fetch_optional(&state.db)
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
                &state.db,
                &state.redis,
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
                sqlx::query_scalar::<_, Option<String>>(
                    r#"
                    SELECT role::text AS role
                    FROM workspace_memberships
                    WHERE workspace_id = $1
                      AND principal_id = $2
                      AND status = 'active'
                    LIMIT 1
                    "#,
                )
                .bind(workspace_id)
                .bind(subject_id)
                .fetch_optional(&state.db)
                .await?
                .flatten()
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
    ) = resolve_actor_context(&state.db, claims.act.as_ref()).await?;

    let mut network_valid = None;
    if let Some(t_id) = tenant_id {
        let allowed_ips_row = sqlx::query("SELECT allowed_ips FROM tenants WHERE id = $1 LIMIT 1")
            .bind(t_id)
            .fetch_optional(&state.db)
            .await?;

        if let Some(row) = allowed_ips_row {
            let allowed_ips: Option<Vec<String>> = row.get("allowed_ips");
            if let Some(ips) = allowed_ips {
                if !ips.is_empty() {
                    let mut valid = false;
                    if let Some(ref ip_str) = client_ip {
                        if let Ok(client_addr) = ip_str.parse::<std::net::IpAddr>() {
                            for cidr in ips {
                                if let Ok(net) = cidr.parse::<ipnet::IpNet>() {
                                    if net.contains(&client_addr) {
                                        valid = true;
                                        break;
                                    }
                                } else if let Ok(allowed_ip) = cidr.parse::<std::net::IpAddr>() {
                                    if allowed_ip == client_addr {
                                        valid = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    network_valid = Some(valid);
                }
            }
        }
    }

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
            .map(|act| serde_json::to_value(act).ok())
            .flatten(),
        actor_principal_type,
        actor_role,
        actor_workspace_id,
        actor_organization_id,
        actor_tenant_id,
        network_valid,
    })
}

/// Generate tokens for a user.
pub async fn generate_tokens(
    state: &AppState,
    user_id: Uuid,
    tenant_id: Option<Uuid>,
    organization_id: Option<Uuid>,
    workspace_id: Uuid,
    scope: String,
    audience: Option<String>,
    session_id: Option<Uuid>,
) -> Result<TokenView, AppError> {
    let workspace_region = sqlx::query_scalar::<_, Option<String>>(
        "SELECT data_region::text FROM workspaces WHERE id = $1",
    )
    .bind(workspace_id)
    .fetch_optional(&state.db)
    .await?;
    let workspace_region = workspace_region.flatten();

    let tokens = state.jwt.generate_token_pair_with_session(
        user_id,
        Some(workspace_id),
        workspace_region,
        &scope,
        session_id,
        tenant_id,
        organization_id,
        Some("aal1"),
        Some(vec!["pwd".to_string()]),
        None,
        Some(Utc::now().timestamp()),
        None,
    )?;

    Ok(TokenView {
        access_token: tokens.access_token,
        token_type: tokens.token_type,
        expires_in: tokens.expires_in,
        refresh_token: Some(tokens.refresh_token),
        scope: audience.map_or(scope.clone(), |value| format!("{scope} audience:{value}")),
        authorization_details: Vec::new(),
        issued_token_type: None,
    })
}

async fn resolve_actor_context(
    db: &sqlx::PgPool,
    actor_claim: Option<&crate::domains::auth::jwt::types::ActorClaim>,
) -> Result<
    (
        Option<String>,
        Option<String>,
        Option<Uuid>,
        Option<Uuid>,
        Option<Uuid>,
    ),
    AppError,
> {
    let Some(actor_claim) = actor_claim else {
        return Ok((None, None, None, None, None));
    };
    let actor_principal_id = Uuid::parse_str(&actor_claim.sub)
        .map_err(|_| AppError::unauthorized("invalid_grant", "The actor token is invalid."))?;

    let row = sqlx::query(
        r#"
        SELECT
          sa.workspace_id,
          w.organization_id,
          w.tenant_id,
          wm.role::text AS role
        FROM service_accounts sa
        INNER JOIN workspaces w ON w.id = sa.workspace_id
        LEFT JOIN workspace_memberships wm
          ON wm.workspace_id = sa.workspace_id
         AND wm.principal_id = sa.principal_id
         AND wm.status = 'active'
        WHERE sa.principal_id = $1
        LIMIT 1
        "#,
    )
    .bind(actor_principal_id)
    .fetch_optional(db)
    .await?;

    let Some(row) = row else {
        return Ok((Some("service_account".to_string()), None, None, None, None));
    };

    Ok((
        Some("service_account".to_string()),
        row.get("role"),
        row.get("workspace_id"),
        row.get("organization_id"),
        row.get("tenant_id"),
    ))
}
