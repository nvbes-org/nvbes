use super::hosted_store::{delete_hosted_authorization_state, get_hosted_authorization_state};
use super::hosted_types::HostedLoginDecision;
use crate::domains::oauth::authorization_subject::AuthorizationSubject;
use crate::http::error::AppError;

pub(crate) async fn complete_hosted_authorization(
    db: &sqlx::PgPool,
    redis: &nvbes_redis::RedisPool,
    state_id: &str,
    subject: &AuthorizationSubject,
    consent_action: Option<&str>,
) -> Result<HostedLoginDecision, AppError> {
    let lock_key = format!("oauth-hosted-authorization:{state_id}");
    let locked = nvbes_redis::lock::acquire(redis, &lock_key, 15)
        .await
        .map_err(|error| {
            AppError::internal("hosted_authorization_lock_failed", error.to_string())
        })?;
    if !locked {
        return Err(AppError::conflict(
            "hosted_authorization_locked",
            "The authorization request is already being processed.",
        ));
    }

    let result =
        complete_hosted_authorization_with_lock(db, redis, state_id, subject, consent_action).await;
    nvbes_redis::lock::release(redis, &lock_key)
        .await
        .map_err(|error| {
            AppError::internal("hosted_authorization_lock_failed", error.to_string())
        })?;
    result
}

async fn complete_hosted_authorization_with_lock(
    db: &sqlx::PgPool,
    redis: &nvbes_redis::RedisPool,
    state_id: &str,
    subject: &AuthorizationSubject,
    consent_action: Option<&str>,
) -> Result<HostedLoginDecision, AppError> {
    let Some(state) = get_hosted_authorization_state(redis, state_id).await? else {
        return Ok(invalid_state_decision());
    };
    if state.expires_at <= chrono::Utc::now() {
        delete_hosted_authorization_state(redis, state_id).await?;
        return Ok(invalid_state_decision());
    }

    if consent_action == Some("deny") {
        let redirect_url = super::hosted_service::build_oauth_error_redirect_url(
            &state.redirect_uri,
            "access_denied",
            "The resource owner denied the authorization request.",
            state.state.as_deref(),
        )?;
        delete_hosted_authorization_state(redis, state_id).await?;
        return Ok(HostedLoginDecision::Redirect { redirect_url });
    }

    let code = crate::domains::oauth::flows::create_authorization_code(
        db,
        redis,
        subject.auth.user_id,
        subject.auth.session_id,
        crate::domains::oauth::service::CreateAuthorizationCodeInput {
            client_id: state.client_id.clone(),
            user_id: subject.auth.user_id,
            session_id: Some(subject.auth.session_id),
            workspace_id: subject.workspace_id,
            tenant_id: subject.tenant_id,
            organization_id: subject.auth.organization_id,
            scope: state.scope.clone(),
            redirect_uri: state.redirect_uri.clone(),
            nonce: state.nonce.clone(),
            audience: state.audience.clone(),
            resource_indicators: state.resource_indicators.clone(),
            authorization_details: state.authorization_details.clone(),
            code_challenge: state.code_challenge.clone(),
            code_challenge_method: state.code_challenge_method.clone(),
            consent_action: consent_action.map(ToOwned::to_owned),
            dpop_jkt: state.dpop_jkt.clone(),
        },
    )
    .await;

    match code {
        Ok(code) => {
            let mut params = vec![("code", code.code.as_str())];
            if let Some(oauth_state) = state.state.as_deref() {
                params.push(("state", oauth_state));
            }
            let redirect_url =
                super::hosted_service::build_oauth_redirect_url(&state.redirect_uri, &params)?;
            delete_hosted_authorization_state(redis, state_id).await?;
            Ok(HostedLoginDecision::Redirect { redirect_url })
        }
        Err(error) if consent_action.is_none() && error.code == "consent_required" => {
            super::hosted_service::get_hosted_login_decision(db, redis, state_id).await
        }
        Err(error) => {
            let (oauth_error, description) = hosted_oauth_error(&error);
            let redirect_url = super::hosted_service::build_oauth_error_redirect_url(
                &state.redirect_uri,
                oauth_error,
                description,
                state.state.as_deref(),
            )?;
            delete_hosted_authorization_state(redis, state_id).await?;
            Ok(HostedLoginDecision::Redirect { redirect_url })
        }
    }
}

fn invalid_state_decision() -> HostedLoginDecision {
    HostedLoginDecision::ErrorPage {
        code: "invalid_request".to_string(),
        message: "This login request is no longer valid.".to_string(),
    }
}

fn hosted_oauth_error(error: &AppError) -> (&'static str, &'static str) {
    if error.status.is_server_error() {
        return (
            "server_error",
            "The authorization server could not complete the request.",
        );
    }
    match error.code.as_str() {
        "client_scope_not_allowed" => ("invalid_scope", "The requested scope is not allowed."),
        "client_audience_not_allowed" | "client_resource_not_allowed" | "invalid_target" => (
            "invalid_target",
            "The requested resource server is not allowed.",
        ),
        _ => ("access_denied", "The authorization request was denied."),
    }
}

#[cfg(test)]
mod tests {
    use super::hosted_oauth_error;
    use crate::http::error::AppError;

    #[test]
    fn hosted_errors_do_not_disclose_server_details() {
        let error = AppError::internal("redis_error", "redis.internal.example");

        assert_eq!(
            hosted_oauth_error(&error),
            (
                "server_error",
                "The authorization server could not complete the request."
            )
        );
    }
}
