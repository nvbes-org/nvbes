use sqlx::Row;
use sqlx::postgres::PgPool;
use uuid::Uuid;

use crate::domains::auth::jwt::JwtService;
use crate::domains::oauth::service::TokenExchangeInput;
use crate::domains::oauth::service::TokenView;
use crate::http::error::AppError;

pub async fn token_exchange(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    jwt: &JwtService,
    input: TokenExchangeInput,
    token_confirmation: Option<crate::domains::auth::jwt::TokenConfirmation>,
) -> Result<TokenView, AppError> {
    let audience = input
        .audience
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("nvbes-cloud-service");
    if audience != "nvbes-cloud-service" {
        return Err(AppError::bad_request(
            "invalid_audience",
            "Drive delegated tokens must target the nvbes-cloud-service audience.",
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
            tenant_id,
            revoked_at
        FROM oauth_clients
        WHERE client_id = $1
        LIMIT 1
        "#,
    )
    .bind(&input.client_id)
    .fetch_optional(db)
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
    let client_tenant_id: Uuid = client_row.get("tenant_id");
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
            crate::domains::oauth::verify_client_secret_with_overlap(
                client_tenant_id,
                &client_display_id,
                secret,
                &client_secret_hash,
            )
            .await?;
        }
    } else if let Some(ref secret) = input.client_secret {
        crate::domains::oauth::verify_client_secret_with_overlap(
            client_tenant_id,
            &client_display_id,
            secret,
            &client_secret_hash,
        )
        .await?;
    }

    let subject_claims =
        validate_subject_token(redis, jwt, &input.subject_token, &input.subject_token_type).await?;

    validate_subject_is_user(&subject_claims)?;

    let (actor_sub, actor_client_id) = if let Some(ref actor_token) = input.actor_token {
        let actor_claims =
            validate_actor_token(jwt, actor_token, input.actor_token_type.as_deref()).await?;

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
                format!(
                    "The requested scope '{}' exceeds the subject token scope.",
                    requested
                ),
            ));
        }
    }
    if let Some(ref actor_token) = input.actor_token {
        let actor_claims =
            validate_actor_token(jwt, actor_token, input.actor_token_type.as_deref()).await?;
        let actor_scopes: Vec<&str> = actor_claims
            .scope
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .collect();
        for requested in &requested_scopes {
            if !actor_scopes.contains(requested) {
                return Err(AppError::forbidden(
                    "invalid_scope",
                    format!(
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
        db,
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

    let access_token = jwt
        .generate_token_exchange(
            &subject_claims,
            &actor_sub,
            actor_client_id.as_deref(),
            scope_str,
            audience,
            token_confirmation.clone(),
        )
        .await?;
    metrics::counter!("identity_oauth_token_exchange_total").increment(1);

    Ok(TokenView {
        access_token,
        token_type: if token_confirmation
            .as_ref()
            .is_some_and(|confirmation| confirmation.jkt.is_some())
        {
            "DPoP".to_string()
        } else {
            "Bearer".to_string()
        },
        expires_in: jwt.access_token_expiry.num_seconds(),
        refresh_token: None,
        id_token: None,
        scope: scope_str.to_string(),
        authorization_details: subject_claims.authorization_details,
        issued_token_type: Some("urn:ietf:params:oauth:token-type:access_token".to_string()),
    })
}

async fn validate_subject_token(
    redis: &nvbes_redis::RedisPool,
    jwt: &JwtService,
    token: &str,
    token_type: &str,
) -> Result<crate::domains::auth::jwt::types::TokenClaims, AppError> {
    if token_type != "urn:ietf:params:oauth:token-type:access_token"
        && token_type != "urn:ietf:params:oauth:token-type:refresh_token"
    {
        return Err(AppError::bad_request(
            "invalid_request",
            format!("Unsupported subject_token_type: {}", token_type),
        ));
    }

    let expected_type = if token_type == "urn:ietf:params:oauth:token-type:refresh_token" {
        "refresh"
    } else {
        "access"
    };

    jwt.validate_token(Some(redis), token, expected_type)
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
    jwt: &JwtService,
    token: &str,
    token_type: Option<&str>,
) -> Result<crate::domains::auth::jwt::types::TokenClaims, AppError> {
    let token_type = token_type.unwrap_or("urn:ietf:params:oauth:token-type:access_token");
    if token_type != "urn:ietf:params:oauth:token-type:access_token" {
        return Err(AppError::bad_request(
            "invalid_request",
            format!("Unsupported actor_token_type: {}", token_type),
        ));
    }

    let claims = jwt.decode_token(token, "access").map_err(|_| {
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
#[path = "identity.domains.oauth.flows.token_exchange.tests.rs"]
mod tests;
