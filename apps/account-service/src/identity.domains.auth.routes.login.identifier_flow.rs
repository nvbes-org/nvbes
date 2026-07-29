use uuid::Uuid;

use crate::domains::auth::mfa_policy::MfaPolicyDecision;
use crate::http::error::AppError;

pub(crate) struct IdentifierChallenge {
    pub principal_id: Option<Uuid>,
    pub next_step: String,
    pub available_methods: Option<Vec<String>>,
}

pub(crate) async fn resolve_uniform_identifier_challenge(
    db: &sqlx::PgPool,
    email: &str,
) -> Result<IdentifierChallenge, AppError> {
    if let Some(policy) =
        crate::domains::auth::sso_policy::required_sso_policy_for_email(db, email).await?
    {
        return Ok(IdentifierChallenge {
            principal_id: None,
            next_step: "sso".to_string(),
            available_methods: Some(vec![policy.provider_type]),
        });
    }

    Ok(IdentifierChallenge {
        principal_id: None,
        next_step: "pwd".to_string(),
        available_methods: None,
    })
}

pub(crate) async fn resolve_mfa_challenge_methods(
    db: &sqlx::PgPool,
    principal_id: Uuid,
) -> Result<Vec<String>, AppError> {
    let methods = crate::domains::auth::mfa::list_login_methods(db, principal_id).await?;
    let privileged =
        crate::domains::auth::mfa_policy::principal_has_privileged_role(db, principal_id).await?;
    crate::domains::auth::mfa_policy::methods_for_privileged_principal(methods, privileged)
}

pub(crate) async fn resolve_post_password_challenge(
    db: &sqlx::PgPool,
    principal_id: Uuid,
    risk_score: f64,
) -> Result<Option<Vec<String>>, AppError> {
    if risk_score > 0.0 {
        return Ok(Some(resolve_mfa_challenge_methods(db, principal_id).await?));
    }

    let has_active_factor = crate::domains::auth::mfa::has_active_factor(db, principal_id).await?;
    let policy_context =
        crate::domains::auth::mfa_policy::fetch_policy_context(db, principal_id, has_active_factor)
            .await?;

    match crate::domains::auth::mfa_policy::evaluate_mfa_policy(&policy_context) {
        MfaPolicyDecision::Optional => Ok(None),
        MfaPolicyDecision::Challenge => {
            Ok(Some(resolve_mfa_challenge_methods(db, principal_id).await?))
        }
        MfaPolicyDecision::EnrollmentRequired => Err(AppError::forbidden(
            "mfa_enrollment_required",
            "Multi-factor authentication is required before this account can sign in.",
        )),
    }
}
