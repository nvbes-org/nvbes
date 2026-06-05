use crate::app::AppState;
use axum::Router;
use axum::{
    extract::{Extension, State},
    routing::get,
};

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
    responses(
        (status = 200, description = "List of MFA factors", body = crate::domains::auth::types::MfaFactorsResult),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn list_mfa_factors(
    State(state): State<AppState>,
    Extension(auth): Extension<crate::http::middleware::jwt::AuthContext>,
) -> Result<axum::Json<crate::domains::auth::types::MfaFactorsResult>, crate::http::error::AppError>
{
    let result = crate::domains::auth::mfa::list_factors(&state.db, auth.user_id).await?;
    Ok(axum::Json(result))
}

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/factors",
            get(list_mfa_factors).layer(axum::middleware::from_fn_with_state(
                state.clone(),
                crate::http::middleware::jwt::jwt_auth_middleware,
            )),
        )
        .merge(totp::router(state))
        .merge(webauthn::router(state))
        .merge(recovery::router(state))
}
