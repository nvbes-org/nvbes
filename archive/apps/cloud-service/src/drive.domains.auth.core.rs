use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::identity::IdentityAuthClient;
use super::identity_sync::{
    parse_identity_auth_time, parse_optional_identity_session_id, sync_identity_user,
};
use super::jwt::IdentityJwtClaims;
use super::types::{AuthContext, AuthPrincipalKind, MeResult, UserView, WorkspaceView};
use crate::http::error::AppError;

pub async fn authenticate(
    db: &PgPool,
    token: &str,
    request_headers: Option<&axum::http::HeaderMap>,
) -> Result<AuthContext, AppError> {
    let verified_claims = super::jwt::verify_identity_access_token(token).await?;
    let client = IdentityAuthClient::from_env()?;
    let claims = client
        .introspect_access_token(token, request_headers)
        .await?;

    if !claims.active {
        return Err(AppError::unauthorized(
            "invalid_token",
            "The access token is invalid or expired.",
        ));
    }

    if claims.network_valid == Some(false) {
        return Err(AppError::forbidden(
            "network_restriction_violated",
            "Access denied: current location does not match network restrictions.",
        ));
    }

    if let (Some(sid), Some(workspace_id)) = (claims.sid.as_deref(), claims.workspace_id) {
        let mut tx = db.begin().await?;
        nvbes_tenancy::set_transaction_rls_context(
            &mut tx,
            nvbes_tenancy::RlsContext {
                principal_id: claims
                    .sub
                    .as_deref()
                    .and_then(|value| Uuid::parse_str(value).ok()),
                user_id: claims
                    .sub
                    .as_deref()
                    .and_then(|value| Uuid::parse_str(value).ok()),
                tenant_id: claims.tenant_id,
                workspace_id: Some(workspace_id),
            },
        )
        .await?;
        let is_revoked = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM sessions WHERE id = $1 AND revoked_at IS NOT NULL)",
        )
        .bind(Uuid::parse_str(sid).unwrap_or_else(|_| Uuid::nil()))
        .fetch_one(&mut *tx)
        .await
        .unwrap_or(false);
        tx.commit().await?;

        if is_revoked {
            super::identity::invalidate_cached_session(sid);
            return Err(AppError::unauthorized(
                "session_revoked",
                "The session has been revoked.",
            ));
        }
    }

    ensure_claim_consistency(&verified_claims, &claims)?;

    let session_id = parse_optional_identity_session_id(claims.sid.as_deref())?;
    let auth_time = parse_identity_auth_time(claims.auth_time)?;
    let user = sync_identity_user(db, &claims).await?;

    Ok(user.into_auth_context(&claims, session_id, auth_time))
}

fn ensure_claim_consistency(
    verified_claims: &IdentityJwtClaims,
    introspected: &super::identity::IdentityIntrospectionResponse,
) -> Result<(), AppError> {
    if introspected.sub.as_deref() != Some(verified_claims.sub.as_str()) {
        return Err(AppError::unauthorized(
            "invalid_token",
            "Identity introspection does not match the verified subject.",
        ));
    }
    if introspected.client_id.as_deref() != verified_claims.client_id.as_deref() {
        return Err(AppError::unauthorized(
            "invalid_token",
            "Identity introspection does not match the verified client.",
        ));
    }
    if introspected.audience.as_deref() != Some(verified_claims.aud.as_str()) {
        return Err(AppError::unauthorized(
            "invalid_token",
            "Identity introspection does not match the verified audience.",
        ));
    }

    ensure_optional_uuid_matches(
        verified_claims.tenant_id.as_deref(),
        introspected.tenant_id,
        "tenant",
    )?;
    ensure_optional_uuid_matches(
        verified_claims.organization_id.as_deref(),
        introspected.organization_id,
        "organization",
    )?;
    ensure_optional_uuid_matches(
        verified_claims.workspace_id.as_deref(),
        introspected.workspace_id,
        "workspace",
    )?;

    let is_machine = verified_claims.amr.iter().any(|method| method == "m2m");
    let expected_principal_type = if is_machine {
        "service_account"
    } else {
        "user"
    };
    if introspected.principal_type.as_deref() != Some(expected_principal_type) {
        return Err(AppError::unauthorized(
            "invalid_token",
            "Identity introspection does not match the verified principal type.",
        ));
    }

    if verified_claims.act.is_some() != introspected.act.is_some() {
        return Err(AppError::unauthorized(
            "invalid_token",
            "Identity introspection does not match the verified delegation context.",
        ));
    }
    if let (Some(actor_claim), Some(introspected_actor)) =
        (verified_claims.act.as_ref(), introspected.act.as_ref())
    {
        if introspected_actor
            .get("sub")
            .and_then(|value| value.as_str())
            != Some(actor_claim.sub.as_str())
        {
            return Err(AppError::unauthorized(
                "invalid_token",
                "Identity introspection does not match the verified actor subject.",
            ));
        }
        if introspected_actor
            .get("client_id")
            .and_then(|value| value.as_str())
            != actor_claim.client_id.as_deref()
        {
            return Err(AppError::unauthorized(
                "invalid_token",
                "Identity introspection does not match the verified actor client.",
            ));
        }
    }

    Ok(())
}

fn ensure_optional_uuid_matches(
    verified: Option<&str>,
    introspected: Option<Uuid>,
    label: &str,
) -> Result<(), AppError> {
    let verified = verified.map(Uuid::parse_str).transpose().map_err(|_| {
        AppError::unauthorized(
            "invalid_token",
            "Identity token contains an invalid context claim.",
        )
    })?;

    if verified != introspected {
        return Err(AppError::unauthorized(
            "invalid_token",
            format!("Identity introspection does not match the verified {label} context."),
        ));
    }

    Ok(())
}

pub async fn get_me(db: &PgPool, auth: &AuthContext) -> Result<MeResult, AppError> {
    let user_id = auth.user_id()?;
    let mut tx = db.begin().await?;
    nvbes_tenancy::set_transaction_rls_context(
        &mut tx,
        nvbes_tenancy::RlsContext {
            principal_id: Some(auth.principal_id),
            user_id: Some(user_id),
            tenant_id: auth.tenant_id,
            workspace_id: auth.workspace_id,
        },
    )
    .await?;
    let user = sqlx::query(
        r#"
        SELECT id, email, display_name, email_verified_at, status::text AS status
        FROM users
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;

    let workspaces = if auth.principal_kind == AuthPrincipalKind::ServiceAccount {
        match auth.workspace_id {
            Some(workspace_id) => {
                let row = sqlx::query(
                    r#"
                    SELECT
                      w.id,
                      w.name,
                      w.workspace_type::text AS workspace_type,
                      w.trial_ends_at
                    FROM workspaces w
                    WHERE w.id = $1
                      AND w.deleted_at IS NULL
                    LIMIT 1
                    "#,
                )
                .bind(workspace_id)
                .fetch_optional(&mut *tx)
                .await?;

                row.into_iter()
                    .map(|row| WorkspaceView {
                        id: row.get("id"),
                        name: row.get("name"),
                        workspace_type: row.get("workspace_type"),
                        role: auth.role.clone().unwrap_or_else(|| "member".to_string()),
                        trial_ends_at: row.get("trial_ends_at"),
                    })
                    .collect()
            }
            None => Vec::new(),
        }
    } else {
        sqlx::query(
            r#"
            SELECT
              w.id,
              w.name,
              w.workspace_type::text AS workspace_type,
              wm.role::text AS role,
              w.trial_ends_at
            FROM workspace_memberships wm
            INNER JOIN workspaces w ON w.id = wm.workspace_id
            WHERE wm.user_id = $1
              AND wm.status = 'active'
              AND w.deleted_at IS NULL
            ORDER BY w.created_at ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|row| WorkspaceView {
            id: row.get("id"),
            name: row.get("name"),
            workspace_type: row.get("workspace_type"),
            role: row.get("role"),
            trial_ends_at: row.get("trial_ends_at"),
        })
        .collect()
    };

    let result = MeResult {
        user: UserView {
            id: user.get("id"),
            email: user.get("email"),
            display_name: user.get("display_name"),
            email_verified_at: auth.email_verified_at,
            status: user.get("status"),
        },
        workspaces,
        current_session_id: Some(auth.session_id),
        current_tenant_id: auth.tenant_id,
        current_organization_id: auth.organization_id,
        current_workspace_id: auth.workspace_id,
    };
    tx.commit().await?;
    Ok(result)
}
