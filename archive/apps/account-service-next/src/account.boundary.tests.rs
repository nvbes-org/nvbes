const CARGO_MANIFEST: &str = include_str!("../Cargo.toml");
const HTTP_ROUTES: &str = include_str!("account.http.rs");
const MIGRATION: &str = include_str!("../migrations/0001_account_baseline.sql");
const PRIVACY_EXPORT: &str = include_str!("account.privacy.db.rs");
const REGISTRATION_PROJECTION: &str = include_str!("account.registration.db.rs");
const PROFILE_PERSISTENCE: &str = include_str!("account.profile.db.rs");
const PROFILE_PROJECTION: &str = include_str!("account.profile.projection.rs");

#[test]
fn account_resource_server_has_no_identity_runtime_dependencies() {
    for forbidden in [
        "nvbes-product-identity",
        "nvbes-redis",
        "jsonwebtoken",
        "NVBES_REDIS",
        "NVBES_DATABASE_URL",
    ] {
        assert!(
            !CARGO_MANIFEST.contains(forbidden),
            "Account Resource Server dependency boundary includes `{forbidden}`"
        );
    }
}

#[test]
fn canonical_router_has_no_legacy_auth_or_legal_prefix() {
    assert!(!HTTP_ROUTES.contains("\"/auth"));
    assert!(!HTTP_ROUTES.contains("\"/legal"));
}

#[test]
fn account_schema_has_no_interservice_foreign_keys() {
    assert!(
        !MIGRATION
            .split_whitespace()
            .any(|token| token.eq_ignore_ascii_case("REFERENCES")),
        "Account schema must not declare SQL REFERENCES clauses"
    );
    assert!(!MIGRATION.contains("identity_"));
}

#[test]
fn privacy_export_reads_only_account_owned_tables() {
    for forbidden in [
        " FROM users",
        " FROM principals",
        " FROM oauth_",
        " FROM user_",
        " FROM mfa_",
        " FROM devices",
    ] {
        assert!(
            !PRIVACY_EXPORT.contains(forbidden),
            "Account export reads forbidden Identity table via `{forbidden}`"
        );
    }
}

#[test]
fn registration_projection_writes_only_account_owned_tables() {
    for required in [
        "account_inbox_events",
        "account_profiles",
        "account_preferences",
        "account_notifications",
        "account_consents",
    ] {
        assert!(
            REGISTRATION_PROJECTION.contains(required),
            "registration projection must write `{required}`"
        );
    }
    for forbidden in [
        " users",
        " principals",
        " user_consents",
        " oauth_",
        " mfa_",
    ] {
        assert!(
            !REGISTRATION_PROJECTION.contains(forbidden),
            "registration projection crosses the Account boundary via `{forbidden}`"
        );
    }
}

#[test]
fn profile_replacement_and_oidc_projection_share_one_transaction() {
    assert!(PROFILE_PERSISTENCE.contains("profile_projection::enqueue_tx"));
    assert!(PROFILE_PERSISTENCE.contains("tx.commit()"));
    assert!(PROFILE_PROJECTION.contains("account.oidc-profile.updated.v1"));
    assert!(PROFILE_PROJECTION.contains("account_outbox_events"));
    assert!(PROFILE_PROJECTION.contains("profile_version"));
    assert!(!PROFILE_PROJECTION.contains("region:"));
    assert!(!PROFILE_PROJECTION.contains("identity_oidc_profile_claims"));
}
