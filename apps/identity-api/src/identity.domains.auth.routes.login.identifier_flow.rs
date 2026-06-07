use uuid::Uuid;

use crate::database::Database;
use crate::http::error::AppError;

pub(crate) struct IdentifierChallenge {
    pub principal_id: Option<Uuid>,
    pub next_step: String,
    pub available_methods: Option<Vec<String>>,
}

pub(crate) async fn resolve_identifier_challenge(
    db: &Database,
    email: &str,
) -> Result<IdentifierChallenge, AppError> {
    let principal_id =
        crate::domains::auth::password::db::find_principal_and_display_name_by_email(db, email)
            .await?
            .map(|(principal_id, _display_name)| principal_id);

    let (next_step, available_methods) = match principal_id {
        Some(principal_id) => resolve_next_step_for_principal(db, principal_id).await,
        None => ("pwd".to_string(), None),
    };

    Ok(IdentifierChallenge {
        principal_id,
        next_step,
        available_methods,
    })
}

async fn resolve_next_step_for_principal(
    db: &Database,
    principal_id: Uuid,
) -> (String, Option<Vec<String>>) {
    let Ok(prefs) = crate::domains::auth::db::fetch_user_preferences(db, principal_id).await else {
        return ("pwd".to_string(), None);
    };

    if !prefs.skip_password {
        return ("pwd".to_string(), None);
    }

    let Ok(methods) = crate::domains::auth::mfa::list_login_methods(db, principal_id).await else {
        return ("pwd".to_string(), None);
    };

    if methods.contains(&"webauthn".to_string()) {
        return ("mfa".to_string(), Some(methods));
    }

    ("pwd".to_string(), None)
}

pub(crate) async fn resolve_mfa_challenge_methods(
    db: &Database,
    principal_id: Uuid,
) -> Result<Vec<String>, AppError> {
    crate::domains::auth::mfa::list_login_methods(db, principal_id).await
}

pub(crate) async fn resolve_post_password_challenge(
    db: &Database,
    principal_id: Uuid,
) -> Result<Option<Vec<String>>, AppError> {
    if !crate::domains::auth::mfa::has_active_factor(db, principal_id).await? {
        return Ok(None);
    }

    Ok(Some(resolve_mfa_challenge_methods(db, principal_id).await?))
}
