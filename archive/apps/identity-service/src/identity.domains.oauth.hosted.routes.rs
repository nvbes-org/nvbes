use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{app::AppState, http::error::AppError};

use super::hosted_service::get_hosted_login_decision;

#[derive(Debug, Deserialize, ToSchema)]
pub struct HostedConsentRequest {
    pub consent_action: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/hosted-login/{state_id}", get(get_hosted_login))
        .route(
            "/hosted-login/{state_id}/authorize",
            post(authorize_hosted_login),
        )
        .route(
            "/hosted-login/{state_id}/consent",
            post(consent_hosted_login),
        )
}

async fn get_hosted_login(
    State(state): State<AppState>,
    Path(state_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    let decision = match resolve_hosted_subject(&state, &headers, &state_id).await? {
        HostedSubjectResolution::Decision(decision) => decision,
        HostedSubjectResolution::Subject(_) => {
            get_hosted_login_decision(&state.db, &state.redis, &state_id).await?
        }
    };
    Ok(Json(serde_json::to_value(decision)?))
}

async fn authorize_hosted_login(
    State(state): State<AppState>,
    Path(state_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    let decision = match resolve_hosted_subject(&state, &headers, &state_id).await? {
        HostedSubjectResolution::Decision(decision) => decision,
        HostedSubjectResolution::Subject(subject) => {
            crate::domains::oauth::hosted_authorization::complete_hosted_authorization(
                &state.db,
                &state.redis,
                &state_id,
                &subject,
                None,
            )
            .await?
        }
    };
    Ok(Json(serde_json::to_value(decision)?))
}

async fn consent_hosted_login(
    State(state): State<AppState>,
    Path(state_id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<HostedConsentRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let consent_action = request.consent_action.as_deref().ok_or_else(|| {
        AppError::bad_request(
            "invalid_consent_action",
            "The consent action must be approve or deny.",
        )
    })?;
    if !matches!(consent_action, "approve" | "deny") {
        return Err(AppError::bad_request(
            "invalid_consent_action",
            "The consent action must be approve or deny.",
        ));
    }
    let decision = match resolve_hosted_subject(&state, &headers, &state_id).await? {
        HostedSubjectResolution::Decision(decision) => decision,
        HostedSubjectResolution::Subject(subject) => {
            crate::domains::oauth::hosted_authorization::complete_hosted_authorization(
                &state.db,
                &state.redis,
                &state_id,
                &subject,
                Some(consent_action),
            )
            .await?
        }
    };
    Ok(Json(serde_json::to_value(decision)?))
}

enum HostedSubjectResolution {
    Decision(crate::domains::oauth::hosted_types::HostedLoginDecision),
    Subject(crate::domains::oauth::authorization_subject::AuthorizationSubject),
}

async fn resolve_hosted_subject(
    state: &AppState,
    headers: &HeaderMap,
    state_id: &str,
) -> Result<HostedSubjectResolution, AppError> {
    let Some(hosted) =
        crate::domains::oauth::hosted_store::get_hosted_authorization_state(&state.redis, state_id)
            .await?
    else {
        return Ok(HostedSubjectResolution::Decision(
            crate::domains::oauth::hosted_types::HostedLoginDecision::ErrorPage {
                code: "invalid_request".to_string(),
                message: "This login request is no longer valid.".to_string(),
            },
        ));
    };
    let subject = crate::domains::oauth::authorization_subject::authenticate_authorization_subject(
        &state.db,
        &state.redis,
        &state.jwt,
        headers,
        None,
        &hosted.client_id,
    )
    .await;
    match subject {
        Ok(subject) => Ok(HostedSubjectResolution::Subject(subject)),
        Err(error) if error.status == StatusCode::UNAUTHORIZED => {
            let login_url = crate::domains::oauth::hosted_service::build_hosted_login_url(
                &state.config.web_base_url,
                state_id,
            );
            Ok(HostedSubjectResolution::Decision(
                crate::domains::oauth::hosted_types::HostedLoginDecision::LoginRequired {
                    login_url,
                    state_id: state_id.to_string(),
                },
            ))
        }
        Err(error) => Err(error),
    }
}
