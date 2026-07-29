use utoipa::{
    Modify,
    openapi::{
        Components, OpenApi, Required,
        path::{Operation, ParameterBuilder, ParameterIn},
        schema::{ObjectBuilder, Type},
        security::{
            ApiKey, ApiKeyValue, AuthorizationCode, ClientCredentials, Flow, HttpAuthScheme,
            HttpBuilder, OAuth2, Scopes, SecurityScheme,
        },
    },
};

#[path = "identity.http.openapi.security.classification.rs"]
mod classification;

use classification::classify;

pub(super) const BROWSER_SESSION_SCHEME: &str = "browserSession";
pub(super) const OAUTH_CLIENT_BASIC_SCHEME: &str = "oauthClientBasic";
pub(super) const OAUTH_CLIENT_MTLS_SCHEME: &str = "oauthClientMtls";
pub(super) const OAUTH2_SCHEME: &str = "oauth2";
pub(super) const REGISTRATION_ENROLLMENT_SCHEME: &str = "registrationEnrollment";
pub(super) const CSRF_HEADER: &str = "X-CSRF-Token";

pub(super) struct SecurityContract;

impl Modify for SecurityContract {
    fn modify(&self, openapi: &mut OpenApi) {
        add_security_schemes(openapi);
        apply_operation_security(openapi);
    }
}

fn add_security_schemes(openapi: &mut OpenApi) {
    let components = openapi.components.get_or_insert_with(Components::new);
    components.add_security_scheme(
        BROWSER_SESSION_SCHEME,
        SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::with_description(
            "__Host-session",
            "First-party, HttpOnly session cookie. Multi-account sessions use \
             `__Host-session_{authuser}`. Browser mutations also require the signed \
             double-submit `X-CSRF-Token` header.",
        ))),
    );
    components.add_security_scheme(
        REGISTRATION_ENROLLMENT_SCHEME,
        SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::with_description(
            "__Host-registration_enrollment",
            "Short-lived registration enrollment cookie issued after account registration.",
        ))),
    );
    components.add_security_scheme(
        OAUTH_CLIENT_BASIC_SCHEME,
        SecurityScheme::Http(
            HttpBuilder::new()
                .scheme(HttpAuthScheme::Basic)
                .description(Some(
                "OAuth client authentication using `client_secret_basic`. The client ID is the \
                 username and the client secret is the password. The endpoints also accept \
                 `client_secret_post` and `private_key_jwt` through their form fields."
                    .to_string(),
                ))
                .build(),
        ),
    );
    components.add_security_scheme(
        OAUTH_CLIENT_MTLS_SCHEME,
        SecurityScheme::MutualTls {
            description: Some(
                "OAuth mutual-TLS client authentication for clients whose security profile \
                 requires a bound certificate."
                    .to_string(),
            ),
            extensions: None,
        },
    );

    let scopes = oauth_scopes();
    components.add_security_scheme(
        OAUTH2_SCHEME,
        SecurityScheme::OAuth2(OAuth2::with_description(
            [
                Flow::AuthorizationCode(AuthorizationCode::with_refresh_url(
                    "/oauth/authorize",
                    "/oauth/token",
                    scopes.clone(),
                    "/oauth/token",
                )),
                Flow::ClientCredentials(ClientCredentials::new("/oauth/token", scopes)),
            ],
            "OAuth 2.0 access tokens. Account resource operations enforce the scope \
             declared on each operation; workspace resources additionally enforce RBAC.",
        )),
    );
}

fn apply_operation_security(openapi: &mut OpenApi) {
    for (path, path_item) in &mut openapi.paths.paths {
        apply(path, "GET", false, path_item.get.as_mut());
        apply(path, "PUT", true, path_item.put.as_mut());
        apply(path, "POST", true, path_item.post.as_mut());
        apply(path, "DELETE", true, path_item.delete.as_mut());
        apply(path, "PATCH", true, path_item.patch.as_mut());
        apply(path, "OPTIONS", false, path_item.options.as_mut());
        apply(path, "HEAD", false, path_item.head.as_mut());
        apply(path, "TRACE", false, path_item.trace.as_mut());
    }
}

fn apply(path: &str, method: &str, mutating: bool, operation: Option<&mut Operation>) {
    let Some(operation) = operation else {
        return;
    };
    let operation_id = operation
        .operation_id
        .as_deref()
        .unwrap_or_else(|| panic!("{method} {path} must declare an operationId"));
    let security = classify(operation_id).unwrap_or_else(|| {
        panic!("{method} {path} ({operation_id}) has no explicit OpenAPI security classification")
    });
    operation.security = Some(security.requirements());

    if mutating && security.admits_browser_session() {
        add_csrf_header(operation, security.requires_csrf_header());
    }
}

fn add_csrf_header(operation: &mut Operation, required: bool) {
    let parameters = operation.parameters.get_or_insert_with(Vec::new);
    if parameters.iter().any(|parameter| {
        parameter.parameter_in == ParameterIn::Header
            && parameter.name.eq_ignore_ascii_case(CSRF_HEADER)
    }) {
        return;
    }
    parameters.push(
        ParameterBuilder::new()
            .name(CSRF_HEADER)
            .parameter_in(ParameterIn::Header)
            .required(if required {
                Required::True
            } else {
                Required::False
            })
            .description(Some(
                "Required when authenticating with `browserSession`; send the value of the \
                 matching `__Host-csrf_token{_authuser}` cookie. OAuth bearer requests do not \
                 use this header.",
            ))
            .schema(Some(ObjectBuilder::new().schema_type(Type::String)))
            .build(),
    );
}

fn oauth_scopes() -> Scopes {
    Scopes::from_iter(
        crate::domains::oauth::metadata::supported_scopes()
            .into_iter()
            .map(|scope| (scope, scope_description(scope))),
    )
}

fn scope_description(scope: &str) -> &'static str {
    match scope {
        "openid" => "Authenticate with OpenID Connect.",
        "profile" => "Read standard profile claims.",
        "email" => "Read email claims.",
        "offline_access" => "Request refresh-token access.",
        "drive:read" => "Read Drive resources.",
        "drive:write" => "Create and update Drive resources.",
        "drive:admin" => "Administer Drive workspace resources.",
        "account:profile:read" => "Read the account profile.",
        "account:profile:write" => "Update the account profile.",
        "account:email:read" => "Read account email addresses.",
        "account:email:write" => "Manage account email addresses.",
        "account:session:read" => "Read active account sessions.",
        "account:session:write" => "Revoke or confirm account sessions.",
        "account:security:read" => "Read account security configuration.",
        "account:security:write" => "Manage account security configuration.",
        "account:preferences:read" => "Read account preferences.",
        "account:preferences:write" => "Update account preferences.",
        "account:export" => "Request and download an account data export.",
        "account:delete" => "Delete the account.",
        "account:legal:read" => "Read account consent records.",
        "account:legal:write" => "Grant or revoke account consent.",
        "account:oauth-clients:read" => "Read managed OAuth clients and policies.",
        "account:oauth-clients:write" => "Manage OAuth clients and policies.",
        "account:oauth:approve" => "Approve or deny OAuth device authorization.",
        _ => "Access an Account service resource.",
    }
}
