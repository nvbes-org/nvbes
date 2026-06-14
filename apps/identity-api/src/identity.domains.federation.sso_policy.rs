use crate::http::error::AppError;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequiredSsoPolicy {
    pub tenant_id: Uuid,
    pub domain: String,
    pub provider_id: Uuid,
    pub provider_type: String,
}

pub fn required_sso_domain_from_email(email: &str) -> Result<String, AppError> {
    let email = nvbes_core::auth::normalize_email(email);
    nvbes_core::auth::validate_email(&email)?;
    let domain = email
        .split_once('@')
        .map(|(_, domain)| domain.trim().trim_end_matches('.').to_ascii_lowercase())
        .filter(|domain| !domain.is_empty())
        .ok_or_else(|| AppError::bad_request("validation_failed", "The email address is invalid."))?;

    super::validation::validate_domain(&domain)?;
    Ok(domain)
}

pub async fn required_sso_policy_for_email(
    db: &PgPool,
    email: &str,
) -> Result<Option<RequiredSsoPolicy>, AppError> {
    let domain = required_sso_domain_from_email(email)?;
    let rows = sqlx::query(
        r#"
        SELECT
          d.tenant_id,
          d.domain,
          d.sso_provider_id,
          p.provider_type::text AS provider_type
        FROM tenant_domains d
        INNER JOIN federated_identity_providers p
          ON p.id = d.sso_provider_id
         AND p.tenant_id = d.tenant_id
        WHERE lower(d.domain) = $1
          AND d.verified_at IS NOT NULL
          AND d.sso_required = TRUE
          AND d.sso_provider_id IS NOT NULL
          AND p.status = 'active'
        LIMIT 2
        "#,
    )
    .bind(&domain)
    .fetch_all(db)
    .await?;

    if rows.len() > 1 {
        return Err(AppError::conflict(
            "sso_domain_ambiguous",
            "More than one tenant requires SSO for this email domain.",
        ));
    }

    Ok(rows.into_iter().next().map(|row| RequiredSsoPolicy {
        tenant_id: row.get("tenant_id"),
        domain: row.get("domain"),
        provider_id: row.get("sso_provider_id"),
        provider_type: row.get("provider_type"),
    }))
}

pub async fn ensure_password_allowed_for_email(
    db: &PgPool,
    email: &str,
) -> Result<(), AppError> {
    if required_sso_policy_for_email(db, email).await?.is_some() {
        return Err(AppError::forbidden(
            "sso_required",
            "This email domain requires single sign-on.",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::required_sso_domain_from_email;

    #[test]
    fn required_sso_domain_from_email_normalizes_company_domain() {
        let domain = required_sso_domain_from_email(" Alice@Company.COM ")
            .expect("email domain should normalize");

        assert_eq!(domain, "company.com");
    }
}
