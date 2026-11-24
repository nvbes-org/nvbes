use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post, put},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::ConsentError;
use crate::models::{
    ReconcileConsentRequest, RecordAgreementRequest, UpdateCookiePreferencesRequest,
};
use crate::port::ConsentPort;
use crate::service::ConsentService;

const SUBJECT_HEADER: &str = "x-nvbes-subject-id";
const CLIENT_IP_HEADER: &str = "x-nvbes-client-ip";
const USER_AGENT_HEADER: &str = "user-agent";
const SEC_GPC_HEADER: &str = "sec-gpc";

#[derive(Debug, Deserialize)]
pub struct ConsentQuery {
    pub subject_id: Option<Uuid>,
}

fn resolve_subject_id(headers: &HeaderMap, query: &ConsentQuery) -> Uuid {
    if let Some(parsed) = headers
        .get(SUBJECT_HEADER)
        .and_then(|val| val.to_str().ok())
        .and_then(|str_val| Uuid::parse_str(str_val.trim()).ok())
    {
        return parsed;
    }
    if let Some(id) = query.subject_id {
        return id;
    }
    Uuid::nil()
}

fn detect_gpc(headers: &HeaderMap) -> bool {
    headers
        .get(SEC_GPC_HEADER)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|val| val.trim() == "1")
}

pub fn consent_routes(service: Arc<ConsentService>) -> Router {
    Router::new()
        .route("/", get(get_consent_status))
        .route("/legal", post(record_legal_agreement))
        .route("/cookies", put(update_cookies))
        .route("/reconcile", post(reconcile_consent))
        .with_state(service)
}

async fn get_consent_status(
    State(service): State<Arc<ConsentService>>,
    headers: HeaderMap,
    Query(query): Query<ConsentQuery>,
) -> Result<impl IntoResponse, ConsentError> {
    let subject_id = resolve_subject_id(&headers, &query);
    let status = service.get_consent_status(subject_id).await?;

    Ok((StatusCode::OK, Json(status)))
}

async fn record_legal_agreement(
    State(service): State<Arc<ConsentService>>,
    headers: HeaderMap,
    Query(query): Query<ConsentQuery>,
    Json(payload): Json<RecordAgreementRequest>,
) -> Result<impl IntoResponse, ConsentError> {
    let subject_id = resolve_subject_id(&headers, &query);
    if subject_id == Uuid::nil() {
        return Err(ConsentError::InvalidInput(
            "Subject ID required to record agreement".to_string(),
        ));
    }

    let ip = headers
        .get(CLIENT_IP_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::trim);

    let user_agent = headers
        .get(USER_AGENT_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::trim);

    let recorded = service
        .record_agreement(
            subject_id,
            &payload.document_type,
            &payload.document_version,
            ip,
            user_agent,
        )
        .await?;

    Ok((StatusCode::CREATED, Json(recorded)))
}

async fn update_cookies(
    State(service): State<Arc<ConsentService>>,
    headers: HeaderMap,
    Query(query): Query<ConsentQuery>,
    Json(payload): Json<UpdateCookiePreferencesRequest>,
) -> Result<impl IntoResponse, ConsentError> {
    let subject_id = resolve_subject_id(&headers, &query);
    if subject_id == Uuid::nil() {
        return Err(ConsentError::InvalidInput(
            "Subject ID required to update cookie preferences".to_string(),
        ));
    }

    let gpc_signal = detect_gpc(&headers);

    let prefs = service
        .upsert_cookie_preferences(subject_id, payload.analytics, payload.marketing, gpc_signal)
        .await?;

    Ok((StatusCode::OK, Json(prefs)))
}

async fn reconcile_consent(
    State(service): State<Arc<ConsentService>>,
    headers: HeaderMap,
    Query(query): Query<ConsentQuery>,
    Json(payload): Json<ReconcileConsentRequest>,
) -> Result<impl IntoResponse, ConsentError> {
    let authenticated_id = resolve_subject_id(&headers, &query);
    if authenticated_id == Uuid::nil() {
        return Err(ConsentError::InvalidInput(
            "Authenticated subject ID required".to_string(),
        ));
    }

    service
        .reconcile_anonymous_consent(payload.anonymous_subject_id, authenticated_id)
        .await?;

    Ok((StatusCode::NO_CONTENT, ()))
}
