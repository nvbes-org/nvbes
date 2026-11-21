use async_trait::async_trait;
use chrono::Utc;
use hex;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db;
use crate::error::ConsentError;
use crate::models::{
    ConsentStatusResponse, CookiePreferences, CookiePreferencesResponse, DOC_PRIVACY_POLICY,
    DOC_TERMS_OF_SERVICE, LegalAgreement, LegalAgreementSummary,
};
use crate::port::ConsentPort;

#[derive(Clone)]
pub struct ConsentService {
    pool: PgPool,
    salt: String,
}

impl ConsentService {
    pub fn new(pool: PgPool, salt: impl Into<String>) -> Self {
        Self {
            pool,
            salt: salt.into(),
        }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub fn hash_ip(&self, ip: Option<&str>) -> String {
        match ip {
            Some(addr) if !addr.trim().is_empty() => {
                let mut hasher = Sha256::new();
                hasher.update(self.salt.as_bytes());
                hasher.update(b":");
                hasher.update(addr.trim().as_bytes());
                hex::encode(hasher.finalize())
            }
            _ => "0".repeat(64),
        }
    }
}

#[async_trait]
impl ConsentPort for ConsentService {
    async fn has_accepted_latest(
        &self,
        subject_id: Uuid,
        document_type: &str,
        required_version: &str,
    ) -> Result<bool, ConsentError> {
        db::has_accepted_version(&self.pool, subject_id, document_type, required_version).await
    }

    async fn record_agreement(
        &self,
        subject_id: Uuid,
        document_type: &str,
        document_version: &str,
        ip: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<LegalAgreement, ConsentError> {
        if document_type != DOC_TERMS_OF_SERVICE && document_type != DOC_PRIVACY_POLICY {
            return Err(ConsentError::UnsupportedDocument(document_type.to_string()));
        }
        if document_version.trim().is_empty() {
            return Err(ConsentError::InvalidInput(
                "document_version cannot be empty".to_string(),
            ));
        }

        let agreement = LegalAgreement {
            id: Uuid::new_v4(),
            subject_id,
            document_type: document_type.to_string(),
            document_version: document_version.trim().to_string(),
            agreed_at: Utc::now(),
            ip_hash: self.hash_ip(ip),
            user_agent: user_agent.map(str::to_string),
        };

        let saved = db::insert_legal_agreement(&self.pool, &agreement).await?;

        let details = json!({
            "document_type": saved.document_type,
            "document_version": saved.document_version,
            "ip_hash": saved.ip_hash,
        });
        db::insert_audit_event(&self.pool, subject_id, "legal_agreement_recorded", &details)
            .await?;

        Ok(saved)
    }

    async fn get_cookie_preferences(
        &self,
        subject_id: Uuid,
    ) -> Result<CookiePreferences, ConsentError> {
        let existing = db::get_cookie_preferences(&self.pool, subject_id).await?;
        Ok(existing.unwrap_or_else(|| CookiePreferences {
            subject_id,
            ..Default::default()
        }))
    }

    async fn upsert_cookie_preferences(
        &self,
        subject_id: Uuid,
        analytics: bool,
        marketing: bool,
        gpc_signal: bool,
    ) -> Result<CookiePreferences, ConsentError> {
        // Enforce Global Privacy Control: if GPC header is detected, force opt-out
        let (effective_analytics, effective_marketing) = if gpc_signal {
            (false, false)
        } else {
            (analytics, marketing)
        };

        let prefs = CookiePreferences {
            id: Uuid::new_v4(),
            subject_id,
            functional: true,
            analytics: effective_analytics,
            marketing: effective_marketing,
            gpc_signal,
            updated_at: Utc::now(),
        };

        let saved = db::upsert_cookie_preferences(&self.pool, &prefs).await?;

        let details = json!({
            "functional": saved.functional,
            "analytics": saved.analytics,
            "marketing": saved.marketing,
            "gpc_signal": saved.gpc_signal,
        });
        db::insert_audit_event(
            &self.pool,
            subject_id,
            "cookie_preferences_updated",
            &details,
        )
        .await?;

        Ok(saved)
    }

    async fn get_consent_status(
        &self,
        subject_id: Uuid,
    ) -> Result<ConsentStatusResponse, ConsentError> {
        let agreements = db::get_latest_legal_agreements(&self.pool, subject_id).await?;
        let cookies = self.get_cookie_preferences(subject_id).await?;

        let agreement_summaries = agreements
            .into_iter()
            .map(|a| LegalAgreementSummary {
                document_type: a.document_type,
                document_version: a.document_version,
                agreed_at: a.agreed_at,
            })
            .collect();

        Ok(ConsentStatusResponse {
            subject_id,
            cookies: CookiePreferencesResponse {
                functional: cookies.functional,
                analytics: cookies.analytics,
                marketing: cookies.marketing,
                gpc_enforced: cookies.gpc_signal,
            },
            legal_agreements: agreement_summaries,
        })
    }

    async fn reconcile_anonymous_consent(
        &self,
        anonymous_id: Uuid,
        authenticated_id: Uuid,
    ) -> Result<(), ConsentError> {
        db::reconcile_subject(&self.pool, anonymous_id, authenticated_id).await?;

        let details = json!({
            "anonymous_id": anonymous_id,
            "authenticated_id": authenticated_id,
        });
        db::insert_audit_event(
            &self.pool,
            authenticated_id,
            "anonymous_consent_reconciled",
            &details,
        )
        .await?;

        Ok(())
    }
}
