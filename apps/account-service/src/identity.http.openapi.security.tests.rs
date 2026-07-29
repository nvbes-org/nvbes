use std::collections::{BTreeSet, HashSet};

use serde_json::{Value, json};
use utoipa::OpenApi;

use super::IdentityApiDoc;
use super::security::{
    BROWSER_SESSION_SCHEME, CSRF_HEADER, OAUTH_CLIENT_BASIC_SCHEME, OAUTH_CLIENT_MTLS_SCHEME,
    OAUTH2_SCHEME, REGISTRATION_ENROLLMENT_SCHEME,
};

const PUBLIC_OPERATION_IDS: &[&str] = &[
    "change_password_well_known",
    "gpc_well_known",
    "get_jwks",
    "oauth_authorization_server_metadata",
    "openid_configuration",
    "passkey_endpoints_well_known",
    "webauthn_well_known",
    "get_accounts",
    "challenge_identifier",
    "challenge_mfa",
    "challenge_pow",
    "challenge_pwd",
    "challenge_webauthn_start",
    "challenge_webauthn_discoverable_start",
    "challenge_webauthn_discoverable_finish",
    "forgot_password",
    "reset_password",
    "region",
    "supported_regions",
    "register",
    "verify_email",
    "resend_verify_email",
    "authorize",
    "device_authorize",
    "device_verify",
    "token",
];

fn document() -> Value {
    serde_json::to_value(IdentityApiDoc::openapi()).expect("OpenAPI document should serialize")
}

fn operation<'a>(document: &'a Value, path: &str, method: &str) -> &'a Value {
    document["paths"][path][method]
        .as_object()
        .unwrap_or_else(|| panic!("{method} {path} should be documented"));
    &document["paths"][path][method]
}

#[test]
fn security_schemes_publish_exact_oauth_scope_catalog() {
    let document = document();
    let schemes = document["components"]["securitySchemes"]
        .as_object()
        .expect("security schemes should be declared");

    assert_eq!(
        schemes.keys().map(String::as_str).collect::<BTreeSet<_>>(),
        BTreeSet::from([
            BROWSER_SESSION_SCHEME,
            OAUTH_CLIENT_BASIC_SCHEME,
            OAUTH_CLIENT_MTLS_SCHEME,
            OAUTH2_SCHEME,
            REGISTRATION_ENROLLMENT_SCHEME,
        ])
    );
    assert_eq!(schemes[BROWSER_SESSION_SCHEME]["type"], "apiKey");
    assert_eq!(schemes[BROWSER_SESSION_SCHEME]["in"], "cookie");
    assert_eq!(schemes[BROWSER_SESSION_SCHEME]["name"], "__Host-session");
    assert_eq!(
        schemes[REGISTRATION_ENROLLMENT_SCHEME]["name"],
        "__Host-registration_enrollment"
    );
    assert_eq!(schemes[OAUTH_CLIENT_BASIC_SCHEME]["type"], "http");
    assert_eq!(schemes[OAUTH_CLIENT_BASIC_SCHEME]["scheme"], "basic");
    assert_eq!(schemes[OAUTH_CLIENT_MTLS_SCHEME]["type"], "mutualTLS");

    let oauth = &schemes[OAUTH2_SCHEME];
    assert_eq!(
        oauth["flows"]["authorizationCode"]["authorizationUrl"],
        "/oauth/authorize"
    );
    assert_eq!(
        oauth["flows"]["authorizationCode"]["tokenUrl"],
        "/oauth/token"
    );
    assert_eq!(
        oauth["flows"]["clientCredentials"]["tokenUrl"],
        "/oauth/token"
    );

    let declared_scopes = oauth["flows"]["authorizationCode"]["scopes"]
        .as_object()
        .expect("authorization code scopes should be declared")
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let expected_scopes = crate::domains::oauth::metadata::supported_scopes()
        .into_iter()
        .collect::<BTreeSet<_>>();
    assert_eq!(declared_scopes, expected_scopes);
    assert_eq!(
        oauth["flows"]["clientCredentials"]["scopes"],
        oauth["flows"]["authorizationCode"]["scopes"]
    );
}

#[test]
fn every_operation_has_an_explicit_valid_security_contract() {
    let document = document();
    let schemes = document["components"]["securitySchemes"]
        .as_object()
        .expect("security schemes should be declared");
    let oauth_scopes = schemes[OAUTH2_SCHEME]["flows"]["authorizationCode"]["scopes"]
        .as_object()
        .expect("OAuth scopes should be declared");
    let mut operation_ids = HashSet::new();

    for (path, path_item) in document["paths"]
        .as_object()
        .expect("paths should be declared")
    {
        for method in [
            "get", "put", "post", "delete", "patch", "options", "head", "trace",
        ] {
            let Some(operation) = path_item.get(method) else {
                continue;
            };
            let operation_id = operation["operationId"]
                .as_str()
                .unwrap_or_else(|| panic!("{method} {path} has no operationId"));
            assert!(
                operation_ids.insert(operation_id),
                "duplicate operationId: {operation_id}"
            );
            let requirements = operation["security"]
                .as_array()
                .unwrap_or_else(|| panic!("{method} {path} has no explicit security"));
            let anonymous = requirements.is_empty()
                || requirements.iter().any(|requirement| {
                    requirement
                        .as_object()
                        .is_some_and(serde_json::Map::is_empty)
                });
            assert_eq!(
                anonymous,
                PUBLIC_OPERATION_IDS.contains(&operation_id),
                "{method} {path} ({operation_id}) anonymous-access contract drifted"
            );

            for requirement in requirements {
                for (scheme, scopes) in requirement
                    .as_object()
                    .expect("security requirement should be an object")
                {
                    assert!(
                        schemes.contains_key(scheme),
                        "{method} {path} references unknown scheme {scheme}"
                    );
                    if scheme == OAUTH2_SCHEME {
                        for scope in scopes.as_array().expect("OAuth scopes should be an array") {
                            let scope = scope.as_str().expect("OAuth scope should be text");
                            assert!(
                                oauth_scopes.contains_key(scope),
                                "{method} {path} references unknown OAuth scope {scope}"
                            );
                        }
                    } else {
                        assert_eq!(
                            scopes,
                            &json!([]),
                            "{method} {path} assigns scopes to non-OAuth scheme {scheme}"
                        );
                    }
                }
            }
        }
    }

    assert_eq!(
        PUBLIC_OPERATION_IDS.iter().copied().collect::<HashSet<_>>(),
        operation_ids
            .iter()
            .copied()
            .filter(|operation_id| PUBLIC_OPERATION_IDS.contains(operation_id))
            .collect(),
        "public operation allowlist contains an operation absent from the document"
    );
}

#[test]
fn sentinels_distinguish_public_browser_enrollment_and_oauth_access() {
    let document = document();
    assert_eq!(
        operation(&document, "/auth/register", "post")["security"],
        json!([])
    );
    assert_eq!(
        operation(&document, "/auth/verify-email/change", "post")["security"],
        json!([{ (REGISTRATION_ENROLLMENT_SCHEME): [] }])
    );
    assert_eq!(
        operation(&document, "/auth/logout", "post")["security"],
        json!([{ (BROWSER_SESSION_SCHEME): [] }])
    );
    assert_eq!(
        operation(&document, "/auth/me", "get")["security"],
        json!([
            { (BROWSER_SESSION_SCHEME): [] },
            { (OAUTH2_SCHEME): ["account:profile:read"] },
        ])
    );
    assert_eq!(
        operation(&document, "/auth/me", "patch")["security"],
        json!([
            { (BROWSER_SESSION_SCHEME): [] },
            { (OAUTH2_SCHEME): ["account:profile:write"] },
        ])
    );
    assert_eq!(
        operation(&document, "/oauth/userinfo", "get")["security"],
        json!([{ (OAUTH2_SCHEME): [] }])
    );
    assert_eq!(
        operation(&document, "/authz/decision", "post")["security"],
        json!([{ (OAUTH2_SCHEME): [] }])
    );
    for path in ["/oauth/introspect", "/oauth/revoke"] {
        assert_eq!(
            operation(&document, path, "post")["security"],
            json!([
                { (OAUTH_CLIENT_BASIC_SCHEME): [] },
                { (OAUTH_CLIENT_MTLS_SCHEME): [] },
            ])
        );
    }
}

#[test]
fn browser_authenticated_mutations_document_signed_double_submit_csrf() {
    let document = document();
    for (path, path_item) in document["paths"]
        .as_object()
        .expect("paths should be declared")
    {
        for method in ["post", "put", "patch", "delete"] {
            let Some(operation) = path_item.get(method) else {
                continue;
            };
            let admits_browser = operation["security"]
                .as_array()
                .expect("security should be declared")
                .iter()
                .any(|requirement| requirement.get(BROWSER_SESSION_SCHEME).is_some());
            let csrf = operation["parameters"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|parameter| parameter["name"] == CSRF_HEADER);

            assert_eq!(
                csrf.is_some(),
                admits_browser,
                "{method} {path} CSRF documentation does not match browser authentication"
            );
            if let Some(csrf) = csrf {
                assert_eq!(csrf["in"], "header");
                assert_eq!(csrf["schema"]["type"], "string");
            }
        }
    }

    assert_eq!(
        operation(&document, "/auth/logout", "post")["parameters"][0]["required"],
        true
    );
    let profile_parameters = operation(&document, "/auth/me", "patch")["parameters"]
        .as_array()
        .expect("profile mutation parameters should be declared");
    let profile_csrf = profile_parameters
        .iter()
        .find(|parameter| parameter["name"] == CSRF_HEADER)
        .expect("profile mutation should document CSRF");
    assert_eq!(profile_csrf["required"], false);
}

#[test]
fn profile_avatar_runtime_surface_is_present_in_the_contract() {
    let document = document();
    let avatar = &document["paths"]["/auth/me/avatar"];

    assert!(avatar["get"].is_object());
    assert!(avatar["post"].is_object());
    assert!(avatar["delete"].is_object());
}
