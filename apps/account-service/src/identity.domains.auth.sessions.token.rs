use uuid::Uuid;

use crate::domains::auth::password;
use crate::http::error::AppError;

const TOKEN_VERSION: &str = "v1";

pub fn issue(session_id: Uuid) -> String {
    format!(
        "{TOKEN_VERSION}.{session_id}.{}",
        password::generate_random_token()
    )
}

pub fn session_id(token: &str) -> Result<Uuid, AppError> {
    let mut parts = token.split('.');
    let version = parts.next();
    let session_id = parts.next();
    let secret = parts.next();

    if version != Some(TOKEN_VERSION)
        || parts.next().is_some()
        || secret.is_none_or(|value| value.len() < 32)
    {
        return Err(AppError::unauthorized(
            "invalid_session_cookie",
            "The browser session cookie is invalid.",
        ));
    }

    Uuid::parse_str(session_id.unwrap_or_default()).map_err(|_| {
        AppError::unauthorized(
            "invalid_session_cookie",
            "The browser session cookie is invalid.",
        )
    })
}

pub fn hash(token: &str) -> String {
    password::token_hash(token)
}

#[cfg(test)]
mod tests {
    use super::{issue, session_id};
    use uuid::Uuid;

    #[test]
    fn issued_token_is_versioned_and_bound_to_its_session() {
        let id = Uuid::new_v4();
        let token = issue(id);

        assert_eq!(session_id(&token).unwrap(), id);
        assert!(token.starts_with("v1."));
        assert_eq!(token.split('.').count(), 3);
    }

    #[test]
    fn rejects_invalid_session_cookie_shape() {
        assert!(session_id("v1.not-a-uuid.short").is_err());
    }
}
