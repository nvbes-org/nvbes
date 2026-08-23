use axum::http::HeaderMap;

use crate::app::AppState;
use crate::domains::auth::sessions;
use crate::http::error::AppError;

pub async fn authenticate_session_request(
    state: &AppState,
    headers: &HeaderMap,
    authuser: &str,
) -> Result<crate::domains::auth::types::AuthContext, AppError> {
    if let Some(token) = crate::http::request::authorization_bearer_token(headers)? {
        return sessions::authenticate_with_request(
            &state.db,
            &state.redis,
            &state.jwt,
            &token,
            headers,
        )
        .await
        .map_err(require_reauthentication_for_unauthorized);
    }

    let cookie = crate::http::request::browser_session_token_with_authuser(headers, authuser)?;
    sessions::authenticate_browser_session(&state.db, &state.redis, &cookie, headers)
        .await
        .map_err(require_reauthentication_for_unauthorized)
}

fn require_reauthentication_for_unauthorized(error: AppError) -> AppError {
    if error.status == axum::http::StatusCode::UNAUTHORIZED {
        error.requiring_reauthentication()
    } else {
        error
    }
}

#[cfg(test)]
mod tests {
    use super::require_reauthentication_for_unauthorized;
    use crate::http::error::AppError;
    use nvbes_core::http::error::ErrorRecovery;

    #[test]
    fn terminal_session_authentication_errors_require_reauthentication() {
        let error = require_reauthentication_for_unauthorized(AppError::unauthorized(
            "session_not_found",
            "Session not found.",
        ));

        assert_eq!(error.recovery, Some(ErrorRecovery::Reauthenticate));
    }
}
