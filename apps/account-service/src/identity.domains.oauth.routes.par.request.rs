use crate::http::error::AppError;

use super::ParRequest;

pub(super) fn validate_par_request(request: &ParRequest) -> Result<(), AppError> {
    let redirect_uri = request
        .redirect_uri
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_string();

    if redirect_uri.is_empty() {
        return Err(AppError::bad_request(
            "missing_redirect_uri",
            "The redirect_uri parameter is required for PAR.",
        ));
    }

    Ok(())
}
