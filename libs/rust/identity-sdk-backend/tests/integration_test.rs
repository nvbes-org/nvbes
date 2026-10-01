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

fn sample_user() -> UserView {
    UserView {
        id: "usr_123".to_string(),
        email: "test@nvbes.fr".to_string(),
        display_name: "Test User".to_string(),
        firstname: None,
        lastname: None,
        username: Some("testuser".to_string()),
        birthdate: None,
        region: None,
        email_verified: true,
        mfa_enabled: true,
        created_at: "2025-01-01T00:00:00Z".to_string(),
    }
}

fn sample_session() -> SessionView {
    SessionView {
        id: "sess_123".to_string(),
        tenant_id: None,
        organization_id: None,
        workspace_id: None,
        workspace_region: None,
        created_at: "2025-01-01T00:00:00Z".to_string(),
        last_seen_at: "2025-01-01T00:00:00Z".to_string(),
        expires_at: "2025-01-02T00:00:00Z".to_string(),
        revoked_at: None,
        ip: None,
        user_agent: None,
        current: true,
    }
}

#[tokio::test]
async fn test_identity_client_full_coverage() {
    use axum::{
        routing::{delete, get, post},
        Json, Router,
    };
    use tokio::net::TcpListener;

    let app = Router::new()
        .route(
            "/auth/challenge/identifier",
            post(|| async {
                Json(IdentifierResult {
                    next_step: "password".to_string(),
                    state_token: "st_abc123".to_string(),
                    available_methods: Some(vec!["pwd".to_string()]),
                })
            }),
        )
        .route(
            "/auth/challenge/pwd",
            post(|| async {
                Json(LoginResult {
                    user: sample_user(),
                    session: sample_session(),
                    session_token: Some("stok_123".to_string()),
                    verification_resend_available_at: None,
                })
            }),
        )
        .route(
            "/auth/challenge/mfa",
            post(|| async {
                Json(LoginResult {
                    user: sample_user(),
                    session: sample_session(),
                    session_token: Some("stok_mfa".to_string()),
                    verification_resend_available_at: None,
                })
            }),
        )
        .route(
            "/auth/me",
            get(|| async {
                Json(MeResult {
                    user: sample_user(),
                    current_tenant_id: None,
                    current_organization_id: None,
                    current_workspace_id: None,
                    current_workspace_region: None,
                })
            }),
        )
        .route(
            "/workspaces",
            get(|| async {
                Json(WorkspacesResult {
                    workspaces: vec![WorkspaceView {
                        id: "ws_123".to_string(),
                        name: "Main Workspace".to_string(),
                        workspace_type: "team".to_string(),
                        data_region: "eu-west-1".to_string(),
                        role: "admin".to_string(),
                        trial_ends_at: None,
                        jurisdiction: None,
                        plan_code: None,
                        policy: None,
                        created_at: None,
                        updated_at: None,
                    }],
                })
            }),
        )
        .route("/auth/logout", post(|| async {}))
        .route(
            "/oauth/token",
            post(|| async {
                Json(TokenResponse {
                    access_token: "at_test".to_string(),
                    refresh_token: Some("rt_test".to_string()),
                    token_type: "Bearer".to_string(),
                    expires_in: 3600,
                    scope: "openid profile email".to_string(),
                })
            }),
        )
        .route(
            "/auth/mfa/factors",
            get(|| async {
                Json(MfaFactorsResult {
                    factors: vec![MfaFactorView {
                        id: "f_totp_1".to_string(),
                        factor_type: "totp".to_string(),
                        kind: None,
                        status: "active".to_string(),
                        label: Some("App".to_string()),
                        created_at: "2025-01-01T00:00:00Z".to_string(),
                        confirmed_at: Some("2025-01-01T00:00:00Z".to_string()),
                        last_used_at: None,
                    }],
                    mfa_enabled: true,
                })
            }),
        )
        .route(
            "/auth/mfa/totp/setup",
            post(|| async {
                Json(TotpSetupResult {
                    factor: MfaFactorView {
                        id: "f_totp_pending".to_string(),
                        factor_type: "totp".to_string(),
                        kind: None,
                        status: "pending".to_string(),
                        label: Some("New Totp".to_string()),
                        created_at: "2025-01-01T00:00:00Z".to_string(),
                        confirmed_at: None,
                        last_used_at: None,
                    },
                    secret_base32: "JBSWY3DPEHPK3PXP".to_string(),
                    provisioning_uri: "otpauth://totp/nvbes:test?secret=JBSWY3DPEHPK3PXP"
                        .to_string(),
                })
            }),
        )
        .route(
            "/auth/mfa/totp/confirm",
            post(|| async {
                Json(TotpConfirmResult {
                    factor: MfaFactorView {
                        id: "f_totp_pending".to_string(),
                        factor_type: "totp".to_string(),
                        kind: None,
                        status: "active".to_string(),
                        label: Some("New Totp".to_string()),
                        created_at: "2025-01-01T00:00:00Z".to_string(),
                        confirmed_at: Some("2025-01-01T00:00:00Z".to_string()),
                        last_used_at: None,
                    },
                    mfa_enabled: true,
                })
            }),
        )
        .route(
            "/auth/mfa/webauthn/register/start",
            post(|| async {
                Json(WebauthnRegisterStartResult {
                    factor_id: "f_webauthn_1".to_string(),
                    options: serde_json::json!({ "challenge": "xyz" }),
                })
            }),
        )
        .route(
            "/auth/mfa/webauthn/register/finish",
            post(|| async {
                Json(TotpConfirmResult {
                    factor: MfaFactorView {
                        id: "f_webauthn_1".to_string(),
                        factor_type: "webauthn".to_string(),
                        kind: None,
                        status: "active".to_string(),
                        label: Some("Passkey".to_string()),
                        created_at: "2025-01-01T00:00:00Z".to_string(),
                        confirmed_at: Some("2025-01-01T00:00:00Z".to_string()),
                        last_used_at: None,
                    },
                    mfa_enabled: true,
                })
            }),
        )
        .route("/auth/mfa/factors/{factor_id}", delete(|| async {}))
        .route(
            "/auth/mfa/recovery-codes",
            post(|| async {
                Json(RecoveryCodesResult {
                    codes: vec!["AAAA-1111".to_string(), "BBBB-2222".to_string()],
                })
            }),
        );

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let config = AuthConfig {
        base_url: format!("http://127.0.0.1:{}", port),
        client_id: "client_123".to_string(),
        client_secret: Some("secret_456".to_string()),
    };

    let client = IdentityClient::new(config.clone());
    let _custom_client = IdentityClient::with_http_client(config.clone(), reqwest::Client::new());

    // 1. Auth flows
    let login_res = client
        .login(LoginInput {
            email: "test@nvbes.fr".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(login_res.user.email, "test@nvbes.fr");

    let id_res = client.start_login("test@nvbes.fr").await.unwrap();
    assert_eq!(id_res.next_step, "password");

    let pwd_res = client
        .submit_login_password("st_abc123", "password123")
        .await
        .unwrap();
    matches!(pwd_res, LoginPasswordResult::Success(_));

    let mfa_res = client
        .submit_login_mfa(MfaChallengeInput {
            state_token: "st_abc123".to_string(),
            totp_code: Some("123456".to_string()),
            recovery_code: None,
            webauthn_response: None,
            webauthn_challenge_id: None,
        })
        .await
        .unwrap();
    assert_eq!(mfa_res.user.email, "test@nvbes.fr");

    let me_res = client.get_me(Some("tok_abc")).await.unwrap();
    assert_eq!(me_res.user.email, "test@nvbes.fr");

    let user_info = client.get_user_info("tok_abc").await.unwrap();
    assert_eq!(user_info.email, "test@nvbes.fr");

    let workspaces = client.list_workspaces(Some("tok_abc")).await.unwrap();
    assert_eq!(workspaces.len(), 1);

    client.logout(Some("tok_abc")).await.unwrap();

    // 2. OAuth flows
    let token_res = client
        .exchange_code(
            "code_xyz",
            "https://app.nvbes.fr/callback",
            Some("verifier_123"),
        )
        .await
        .unwrap();
    assert_eq!(token_res.access_token, "at_test");

    let refreshed = client.refresh_token("rt_test").await.unwrap();
    assert_eq!(refreshed.access_token, "at_test");

    // 3. MFA flows
    let factors = client.list_mfa_factors("tok_abc").await.unwrap();
    assert_eq!(factors.factors.len(), 1);

    let setup_totp = client
        .setup_totp("tok_abc", "pwd", Some("App"))
        .await
        .unwrap();
    assert_eq!(setup_totp.factor.id, "f_totp_pending");

    let confirmed_totp = client
        .confirm_totp("tok_abc", "f_totp_pending", "123456")
        .await
        .unwrap();
    assert_eq!(confirmed_totp.factor.status, "active");

    let webauthn_start = client
        .start_webauthn_registration("tok_abc", Some("Key"))
        .await
        .unwrap();
    assert_eq!(webauthn_start.factor_id, "f_webauthn_1");

    let webauthn_finish = client
        .finish_webauthn_registration("tok_abc", "f_webauthn_1", serde_json::json!({}))
        .await
        .unwrap();
    assert_eq!(webauthn_finish.factor.id, "f_webauthn_1");

    client
        .remove_mfa_factor("tok_abc", "f_webauthn_1")
        .await
        .unwrap();

    let rec_codes = client
        .generate_recovery_codes("tok_abc", "password123")
        .await
        .unwrap();
    assert_eq!(rec_codes.codes.len(), 2);
}
