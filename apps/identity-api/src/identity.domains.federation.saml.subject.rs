use chrono::{DateTime, Utc};
use roxmltree::Node;

use crate::http::error::AppError;

const SAML2_BEARER_METHOD: &str = "urn:oasis:names:tc:SAML:2.0:cm:bearer";

#[derive(Debug)]
pub struct SubjectConfirmationData {
    pub not_before: Option<DateTime<Utc>>,
    pub not_on_or_after: Option<DateTime<Utc>>,
    pub recipient: Option<String>,
    pub in_response_to: Option<String>,
}

pub fn validate_subject_confirmation(
    assertion_node: &Node,
    expected_recipient: &str,
) -> Result<SubjectConfirmationData, AppError> {
    let subject = assertion_node
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "Subject")
        .ok_or_else(|| {
            AppError::bad_request(
                "saml_missing_subject",
                "The SAML assertion is missing a Subject element.",
            )
        })?;

    let confirmation = subject
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "SubjectConfirmation")
        .ok_or_else(|| {
            AppError::bad_request(
                "saml_missing_subject_confirmation",
                "The SAML assertion is missing SubjectConfirmation.",
            )
        })?;

    let method = confirmation.attribute("Method").unwrap_or_default();
    if method != SAML2_BEARER_METHOD {
        return Err(AppError::bad_request(
            "saml_invalid_confirmation_method",
            format!(
                "Expected SubjectConfirmation method '{}', got '{}'.",
                SAML2_BEARER_METHOD, method
            ),
        ));
    }

    let confirmation_data = confirmation
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "SubjectConfirmationData")
        .ok_or_else(|| {
            AppError::bad_request(
                "saml_missing_subject_confirmation_data",
                "The SubjectConfirmation is missing SubjectConfirmationData.",
            )
        })?;

    let not_before = confirmation_data
        .attribute("NotBefore")
        .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let not_on_or_after = confirmation_data
        .attribute("NotOnOrAfter")
        .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let recipient = confirmation_data
        .attribute("Recipient")
        .map(|v| v.trim().to_string());

    let in_response_to = confirmation_data
        .attribute("InResponseTo")
        .map(|v| v.trim().to_string());

    if let Some(ref nb) = not_before {
        let now = Utc::now();
        if now < *nb {
            return Err(AppError::bad_request(
                "saml_subject_not_yet_valid",
                "The SubjectConfirmation is not yet valid (NotBefore).",
            ));
        }
    }

    if let Some(ref nooa) = not_on_or_after {
        let now = Utc::now();
        if now >= *nooa {
            return Err(AppError::bad_request(
                "saml_subject_expired",
                "The SubjectConfirmation has expired (NotOnOrAfter).",
            ));
        }
    }

    let recipient = recipient.ok_or_else(|| {
        AppError::bad_request(
            "saml_recipient_missing",
            "The SubjectConfirmation recipient is required.",
        )
    })?;

    if recipient != expected_recipient {
        return Err(AppError::bad_request(
            "saml_recipient_mismatch",
            format!(
                "The SubjectConfirmation recipient '{}' does not match the ACS URL '{}'.",
                recipient, expected_recipient
            ),
        ));
    }

    Ok(SubjectConfirmationData {
        not_before,
        not_on_or_after,
        recipient: Some(recipient),
        in_response_to,
    })
}
