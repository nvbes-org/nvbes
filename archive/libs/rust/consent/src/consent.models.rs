use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

pub const DOC_TERMS_OF_SERVICE: &str = "terms_of_service";
pub const DOC_PRIVACY_POLICY: &str = "privacy_policy";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct LegalAgreement {
    pub id: Uuid,
    pub subject_id: Uuid,
    pub document_type: String,
    pub document_version: String,
    pub agreed_at: DateTime<Utc>,
    pub ip_hash: String,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct CookiePreferences {
    pub id: Uuid,
    pub subject_id: Uuid,
    pub functional: bool,
    pub analytics: bool,
    pub marketing: bool,
    pub gpc_signal: bool,
    pub updated_at: DateTime<Utc>,
}

impl Default for CookiePreferences {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            subject_id: Uuid::nil(),
            functional: true,
            analytics: false,
            marketing: false,
            gpc_signal: false,
            updated_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct LegalAgreementSummary {
    pub document_type: String,
    pub document_version: String,
    pub agreed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CookiePreferencesResponse {
    pub functional: bool,
    pub analytics: bool,
    pub marketing: bool,
    pub gpc_enforced: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ConsentStatusResponse {
    pub subject_id: Uuid,
    pub cookies: CookiePreferencesResponse,
    pub legal_agreements: Vec<LegalAgreementSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct RecordAgreementRequest {
    pub document_type: String,
    pub document_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct UpdateCookiePreferencesRequest {
    pub analytics: bool,
    pub marketing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ReconcileConsentRequest {
    pub anonymous_subject_id: Uuid,
}
