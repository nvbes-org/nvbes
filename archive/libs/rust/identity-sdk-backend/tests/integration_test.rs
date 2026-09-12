use nvbes_identity_sdk::*;

#[test]
fn test_auth_url() {
    let config = AuthConfig {
        base_url: "https://account.nvbes.fr".to_string(),
        client_id: "test-client".to_string(),
        client_secret: None,
    };

    let url = format!("{}/auth/me", config.base_url);
    assert_eq!(url, "https://account.nvbes.fr/auth/me");
}

#[test]
fn test_token_response_serialization() {
    let token = TokenResponse {
        access_token: "test-access-token".to_string(),
        refresh_token: Some("refresh-test".to_string()),
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        scope: "openid profile email".to_string(),
    };

    let json = serde_json::to_value(&token).unwrap();
    assert_eq!(json["token_type"], "Bearer");
    assert_eq!(json["expires_in"], 3600);
    assert!(json["refresh_token"].is_string());
}

#[test]
fn test_token_response_missing_refresh() {
    let token = TokenResponse {
        access_token: "test-access-token".to_string(),
        refresh_token: None,
        token_type: "Bearer".to_string(),
        expires_in: 900,
        scope: "openid".to_string(),
    };

    let json = serde_json::to_value(&token).unwrap();
    assert!(json["refresh_token"].is_null());
}

#[test]
fn test_user_view_deserialization() {
    let json = serde_json::json!({
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "email": "test@nvbes.fr",
        "display_name": "test-user",
        "username": "test-user",
        "email_verified": true,
        "mfa_enabled": false,
        "created_at": "2025-01-01T00:00:00Z",
    });

    let user: UserView = serde_json::from_value(json).unwrap();
    assert_eq!(user.email, "test@nvbes.fr");
    assert!(user.email_verified);
}

#[test]
fn test_mfa_factor_view_serialization() {
    let factor = MfaFactorView {
        id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        factor_type: "totp".to_string(),
        kind: None,
        status: "pending".to_string(),
        label: Some("My Authenticator".to_string()),
        created_at: "2025-01-01T00:00:00Z".to_string(),
        confirmed_at: None,
        last_used_at: None,
    };

    let json = serde_json::to_value(&factor).unwrap();
    assert_eq!(json["factor_type"], "totp");
    assert_eq!(json["status"], "pending");
    assert!(json["label"].is_string());
}

#[test]
fn test_mfa_factors_result() {
    let result = MfaFactorsResult {
        factors: vec![MfaFactorView {
            id: "f1".to_string(),
            factor_type: "totp".to_string(),
            kind: None,
            status: "active".to_string(),
            label: None,
            created_at: "2025-01-01T00:00:00Z".to_string(),
            confirmed_at: Some("2025-01-02T00:00:00Z".to_string()),
            last_used_at: None,
        }],
        mfa_enabled: true,
    };

    let json = serde_json::to_value(&result).unwrap();
    assert!(json["mfa_enabled"].as_bool().unwrap());
    assert_eq!(json["factors"].as_array().unwrap().len(), 1);
}

#[test]
fn test_recovery_codes_result() {
    let result = RecoveryCodesResult {
        codes: vec!["abcd-1234-efgh".to_string(), "ijkl-5678-mnop".to_string()],
    };

    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["codes"].as_array().unwrap().len(), 2);
}

#[test]
fn test_sdk_error_auth() {
    let error = SdkError::auth("Invalid credentials", Some(401));
    let formatted = format!("{}", error);
    assert!(formatted.contains("Invalid credentials"));
    assert!(matches!(error, SdkError::Auth { .. }));
}

#[test]
fn test_sdk_error_token_exchange() {
    let error = SdkError::TokenExchange("invalid_grant".to_string());
    let formatted = format!("{}", error);
    assert!(formatted.contains("invalid_grant"));
}

#[test]
fn test_login_input_serialization() {
    let input = LoginInput {
        email: "test@nvbes.fr".to_string(),
        password: "SecurePass123!".to_string(),
    };

    let json = serde_json::to_value(&input).unwrap();
    assert_eq!(json["email"], "test@nvbes.fr");
    assert_eq!(json["password"], "SecurePass123!");
}

#[test]
fn test_totp_setup_result() {
    let result = TotpSetupResult {
        factor: MfaFactorView {
            id: "f1".to_string(),
            factor_type: "totp".to_string(),
            kind: None,
            status: "pending".to_string(),
            label: Some("Test".to_string()),
            created_at: "2025-01-01T00:00:00Z".to_string(),
            confirmed_at: None,
            last_used_at: None,
        },
        secret_base32: "JBSWY3DPEHPK3PXP".to_string(),
        provisioning_uri: "otpauth://totp/nvbes:test?secret=JBSWY3DPEHPK3PXP".to_string(),
    };

    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["secret_base32"], "JBSWY3DPEHPK3PXP");
    assert!(json["provisioning_uri"]
        .as_str()
        .unwrap()
        .starts_with("otpauth://"));
}
