use uuid::Uuid;

use crate::error::ConsentError;
use crate::models::{
    CookiePreferences, DOC_PRIVACY_POLICY, DOC_TERMS_OF_SERVICE, RecordAgreementRequest,
    UpdateCookiePreferencesRequest,
};
use crate::port::ConsentPort;
use crate::service::ConsentService;

#[test]
fn test_cookie_preferences_default() {
    let prefs = CookiePreferences::default();
    assert!(prefs.functional);
    assert!(!prefs.analytics);
    assert!(!prefs.marketing);
    assert!(!prefs.gpc_signal);
}

#[tokio::test]
async fn test_ip_hashing_consistency() {
    let service = ConsentService::new(
        sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/test")
            .expect("pool"),
        "test-salt-secret",
    );

    let hash1 = service.hash_ip(Some("192.168.1.1"));
    let hash2 = service.hash_ip(Some("192.168.1.1"));
    assert_eq!(hash1, hash2);
    assert_eq!(hash1.len(), 64);
    assert!(!hash1.contains("192.168.1.1"));

    let empty_hash = service.hash_ip(None);
    assert_eq!(empty_hash, "0".repeat(64));

    let whitespace_hash = service.hash_ip(Some("   "));
    assert_eq!(whitespace_hash, "0".repeat(64));
}

#[tokio::test]
async fn test_ip_hashing_salt_isolation() {
    let s1 = ConsentService::new(
        sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/test")
            .expect("pool"),
        "salt-alpha",
    );
    let s2 = ConsentService::new(
        sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/test")
            .expect("pool"),
        "salt-beta",
    );

    let h1 = s1.hash_ip(Some("203.0.113.195"));
    let h2 = s2.hash_ip(Some("203.0.113.195"));
    assert_ne!(h1, h2);
}

#[tokio::test]
async fn test_record_agreement_validation() {
    let service = ConsentService::new(
        sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/test")
            .expect("pool"),
        "salt",
    );

    // Unsupported document type
    let res = service
        .record_agreement(
            Uuid::new_v4(),
            "unsupported_contract",
            "v1.0",
            Some("127.0.0.1"),
            None,
        )
        .await;

    assert!(matches!(res, Err(ConsentError::UnsupportedDocument(_))));

    // Empty version
    let res_empty = service
        .record_agreement(
            Uuid::new_v4(),
            DOC_TERMS_OF_SERVICE,
            "   ",
            Some("127.0.0.1"),
            None,
        )
        .await;

    assert!(matches!(res_empty, Err(ConsentError::InvalidInput(_))));
}

#[test]
fn test_dto_serialization() {
    let req = RecordAgreementRequest {
        document_type: DOC_PRIVACY_POLICY.to_string(),
        document_version: "v2.1.0".to_string(),
    };

    let serialized = serde_json::to_string(&req).expect("serialize");
    let deserialized: RecordAgreementRequest =
        serde_json::from_str(&serialized).expect("deserialize");
    assert_eq!(deserialized.document_type, req.document_type);
    assert_eq!(deserialized.document_version, req.document_version);

    let cookie_req = UpdateCookiePreferencesRequest {
        analytics: true,
        marketing: false,
    };
    let cookie_json = serde_json::to_string(&cookie_req).expect("serialize");
    let cookie_deser: UpdateCookiePreferencesRequest =
        serde_json::from_str(&cookie_json).expect("deserialize");
    assert!(cookie_deser.analytics);
    assert!(!cookie_deser.marketing);
}

#[tokio::test]
async fn test_routes_construction() {
    use crate::routes::consent_routes;
    use std::sync::Arc;

    let service = Arc::new(ConsentService::new(
        sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/test")
            .expect("pool"),
        "salt",
    ));
    let _app = consent_routes(service);
}
