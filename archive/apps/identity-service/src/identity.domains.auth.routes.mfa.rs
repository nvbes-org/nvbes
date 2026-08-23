use crate::app::AppState;
use crate::http::middleware::jwt::account_access::{self, AccountAccess, SECURITY_READ_SCOPE};
use axum::Router;
use axum::{
    extract::{Extension, Query, State},
    routing::get,
};
use serde::Deserialize;

#[path = "identity.domains.auth.routes.mfa.recovery.rs"]
pub mod recovery;
#[path = "identity.domains.auth.routes.mfa.totp.rs"]
pub mod totp;
#[path = "identity.domains.auth.routes.mfa.types.rs"]
pub mod types;
#[path = "identity.domains.auth.routes.mfa.webauthn.rs"]
pub mod webauthn;

pub use recovery::__path_generate_recovery_codes;
pub use recovery::__path_remove_mfa_factor;
pub use totp::__path_begin_totp_enrollment;
pub use totp::__path_confirm_totp_enrollment;
pub use webauthn::__path_begin_webauthn_enrollment;
pub use webauthn::__path_confirm_webauthn_enrollment;
pub use webauthn::__path_webauthn_auth_start;

#[utoipa::path(
    get,
    path = "/auth/mfa/factors",
    tag = "auth",
    params(
        ("limit" = Option<i64>, Query, description = "Max results"),
        ("cursor" = Option<String>, Query, description = "Opaque pagination cursor"),
    ),
    responses(
        (status = 200, description = "List of MFA factors", body = crate::domains::auth::types::MfaFactorsResult),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn list_mfa_factors(
    State(state): State<AppState>,
    Extension(auth): Extension<crate::http::middleware::jwt::AuthContext>,
    Query(query): Query<ListMfaFactorsQuery>,
) -> Result<axum::Json<crate::domains::auth::types::MfaFactorsResult>, crate::http::error::AppError>
{
    let result =
        crate::domains::auth::mfa::list_factors(&state.db, auth.user_id, query.limit, query.cursor)
            .await?;
    Ok(axum::Json(result))
}

#[derive(Debug, Deserialize)]
pub(crate) struct ListMfaFactorsQuery {
    pub limit: Option<i64>,
    pub cursor: Option<String>,
}

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/factors",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(SECURITY_READ_SCOPE),
                get(list_mfa_factors),
            ),
        )
        .merge(totp::router(state))
        .merge(webauthn::router(state))
        .merge(recovery::router(state))
}
