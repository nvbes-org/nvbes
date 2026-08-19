use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ConsentPage {
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConsentInput {
    pub consent_type: String,
    pub document_version: String,
}

#[derive(Debug, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct AccountConsent {
    pub id: Uuid,
    pub principal_id: Uuid,
    pub consent_type: String,
    pub document_version: String,
    pub ip_address: Option<String>,
    pub granted_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ConsentHistory {
    pub consents: Vec<AccountConsent>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

pub(crate) fn validate_input(input: ConsentInput) -> Result<ConsentInput, crate::error::AppError> {
    let consent_type = normalized("consent_type", input.consent_type)?;
    let document_version = normalized("document_version", input.document_version)?;
    Ok(ConsentInput {
        consent_type,
        document_version,
    })
}

fn normalized(field: &'static str, value: String) -> Result<String, crate::error::AppError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 100 {
        return Err(crate::error::AppError::bad_request(
            "invalid_consent",
            format!("`{field}` must contain between 1 and 100 characters."),
        ));
    }
    Ok(value.to_string())
}

pub(crate) fn pagination(page: ConsentPage) -> Result<(Option<Uuid>, i64), crate::error::AppError> {
    let cursor = page
        .cursor
        .map(|value| {
            Uuid::parse_str(&value).map_err(|_| {
                crate::error::AppError::bad_request(
                    "invalid_cursor",
                    "The consent cursor is invalid.",
                )
            })
        })
        .transpose()?;
    let limit = page.limit.unwrap_or(50);
    if !(1..=100).contains(&limit) {
        return Err(crate::error::AppError::bad_request(
            "invalid_limit",
            "The consent page limit must be between 1 and 100.",
        ));
    }
    Ok((cursor, i64::from(limit)))
}

#[cfg(test)]
mod tests {
    use super::{ConsentInput, ConsentPage, pagination, validate_input};

    #[test]
    fn consent_values_and_pagination_are_bounded() {
        let input = validate_input(ConsentInput {
            consent_type: " terms ".to_string(),
            document_version: " v1 ".to_string(),
        })
        .expect("valid consent");
        assert_eq!(input.consent_type, "terms");
        assert!(
            pagination(ConsentPage {
                cursor: None,
                limit: Some(101),
            })
            .is_err()
        );
    }
}
