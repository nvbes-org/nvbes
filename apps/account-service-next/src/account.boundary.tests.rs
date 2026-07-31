const CARGO_MANIFEST: &str = include_str!("../Cargo.toml");
const HTTP_ROUTES: &str = include_str!("account.http.rs");
const MIGRATION: &str = include_str!("../migrations/0001_account_baseline.sql");
const PRIVACY_EXPORT: &str = include_str!("account.privacy.db.rs");

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
    assert!(!MIGRATION.to_ascii_uppercase().contains("REFERENCES"));
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
