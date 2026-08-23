use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const REGISTRATION_EVENT_TYPE: &str = "identity.principal.registered.v1";
const LEGAL_DOCUMENT_VERSION: &str = "2026-06-26";
const REQUIRED_CONSENTS: [&str; 3] = [
    "terms_of_service",
    "privacy_policy",
    "data_processing_agreement",
];

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegistrationProjection {
    pub event_id: Uuid,
    pub event_type: String,
    pub principal_id: Uuid,
    pub marketing_emails_accepted: bool,
    pub consent_ip_address: Option<String>,
    pub consents: Vec<RegistrationConsent>,
    pub registered_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegistrationConsent {
    pub consent_type: String,
    pub document_version: String,
}

impl RegistrationProjection {
    pub fn validate(&mut self) -> Result<(), crate::error::AppError> {
        if self.event_type != REGISTRATION_EVENT_TYPE {
            return Err(crate::error::AppError::bad_request(
                "unsupported_registration_event",
                "The registration event version is not supported.",
            ));
        }
        for consent in &mut self.consents {
            consent.consent_type = bounded("consent_type", &consent.consent_type, 100)?;
            consent.document_version = bounded("document_version", &consent.document_version, 100)?;
        }
        let complete_consent_set = self.consents.len() == REQUIRED_CONSENTS.len()
            && REQUIRED_CONSENTS.iter().all(|required| {
                self.consents.iter().any(|consent| {
                    consent.consent_type == *required
                        && consent.document_version == LEGAL_DOCUMENT_VERSION
                })
            });
        if !complete_consent_set {
            return Err(crate::error::AppError::bad_request(
                "invalid_registration_consents",
                "Registration must include the current complete legal consent set.",
            ));
        }
        Ok(())
    }
}

fn bounded(
    field: &'static str,
    value: &str,
    maximum: usize,
) -> Result<String, crate::error::AppError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > maximum {
        return Err(crate::error::AppError::bad_request(
            "invalid_registration_projection",
            format!("`{field}` must contain between 1 and {maximum} characters."),
        ));
    }
    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    use super::{REGISTRATION_EVENT_TYPE, RegistrationConsent, RegistrationProjection};

    #[test]
    fn projection_requires_the_versioned_event_and_complete_consent_set() {
        let mut projection = RegistrationProjection {
            event_id: Uuid::new_v4(),
            event_type: REGISTRATION_EVENT_TYPE.to_string(),
            principal_id: Uuid::new_v4(),
            marketing_emails_accepted: false,
            consent_ip_address: None,
            consents: [
                "terms_of_service",
                "privacy_policy",
                "data_processing_agreement",
            ]
            .map(|consent_type| RegistrationConsent {
                consent_type: consent_type.to_string(),
                document_version: "2026-06-26".to_string(),
            })
            .into(),
            registered_at: Utc::now(),
        };
        projection.validate().expect("valid projection");
        assert_eq!(projection.consents.len(), 3);
    }

    #[test]
    fn identity_registration_contract_deserializes_without_implicit_fields() {
        let value = json!({
            "event_id": Uuid::new_v4(),
            "event_type": REGISTRATION_EVENT_TYPE,
            "principal_id": Uuid::new_v4(),
            "marketing_emails_accepted": false,
            "consent_ip_address": "203.0.113.0",
            "consents": [
                {"consent_type": "terms_of_service", "document_version": "2026-06-26"},
                {"consent_type": "privacy_policy", "document_version": "2026-06-26"},
                {"consent_type": "data_processing_agreement", "document_version": "2026-06-26"}
            ],
            "registered_at": Utc::now()
        });

        let projection: RegistrationProjection =
            serde_json::from_value(value).expect("Identity registration event contract");
        assert_eq!(projection.event_type, REGISTRATION_EVENT_TYPE);
    }

    #[test]
    fn registration_contract_rejects_legacy_profile_fields() {
        let value = json!({
            "event_id": Uuid::new_v4(),
            "event_type": REGISTRATION_EVENT_TYPE,
            "principal_id": Uuid::new_v4(),
            "username": "legacy-owner",
            "marketing_emails_accepted": false,
            "consent_ip_address": null,
            "consents": [
                {"consent_type": "terms_of_service", "document_version": "2026-06-26"},
                {"consent_type": "privacy_policy", "document_version": "2026-06-26"},
                {"consent_type": "data_processing_agreement", "document_version": "2026-06-26"}
            ],
            "registered_at": Utc::now()
        });

        assert!(serde_json::from_value::<RegistrationProjection>(value).is_err());
    }
}
