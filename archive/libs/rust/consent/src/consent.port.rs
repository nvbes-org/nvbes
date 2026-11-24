use async_trait::async_trait;
use uuid::Uuid;

use crate::error::ConsentError;
use crate::models::{ConsentStatusResponse, CookiePreferences, LegalAgreement};

#[async_trait]
pub trait ConsentPort: Send + Sync {
    async fn has_accepted_latest(
        &self,
        subject_id: Uuid,
        document_type: &str,
        required_version: &str,
    ) -> Result<bool, ConsentError>;

    async fn record_agreement(
        &self,
        subject_id: Uuid,
        document_type: &str,
        document_version: &str,
        ip: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<LegalAgreement, ConsentError>;

    async fn get_cookie_preferences(
        &self,
        subject_id: Uuid,
    ) -> Result<CookiePreferences, ConsentError>;

    async fn upsert_cookie_preferences(
        &self,
        subject_id: Uuid,
        analytics: bool,
        marketing: bool,
        gpc_signal: bool,
    ) -> Result<CookiePreferences, ConsentError>;

    async fn get_consent_status(
        &self,
        subject_id: Uuid,
    ) -> Result<ConsentStatusResponse, ConsentError>;

    async fn reconcile_anonymous_consent(
        &self,
        anonymous_id: Uuid,
        authenticated_id: Uuid,
    ) -> Result<(), ConsentError>;
}
