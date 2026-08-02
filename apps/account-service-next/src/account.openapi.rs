use axum::Json;
use utoipa::{
    Modify, OpenApi,
    openapi::{
        Components,
        security::{AuthorizationCode, Flow, OAuth2, Scopes, SecurityScheme},
    },
};

struct AccountOAuthSecurity;

impl Modify for AccountOAuthSecurity {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let scopes = Scopes::from_iter([
            (
                "account:profile:read",
                "Read the Account profile and avatar.",
            ),
            (
                "account:profile:write",
                "Update the Account profile and avatar.",
            ),
            (
                "account:preferences:read",
                "Read Account display and notification preferences.",
            ),
            (
                "account:preferences:write",
                "Update Account display and notification preferences.",
            ),
            ("account:legal:read", "Read Account consent and GPC state."),
            ("account:legal:write", "Grant or revoke Account consent."),
            ("account:export", "Request and download Account-owned data."),
            ("account:delete", "Request coordinated Account closure."),
            (
                "account:session:read",
                "Read active authentication sessions.",
            ),
            ("account:session:write", "Revoke authentication sessions."),
        ]);
        let components = openapi.components.get_or_insert_with(Components::new);
        components.add_security_scheme(
            "identityOAuth2",
            SecurityScheme::OAuth2(OAuth2::with_description(
                [Flow::AuthorizationCode(
                    AuthorizationCode::with_refresh_url(
                        "https://identity.nvbes.fr/oauth/authorize",
                        "https://identity.nvbes.fr/oauth/token",
                        scopes,
                        "https://identity.nvbes.fr/oauth/token",
                    ),
                )],
                "Identity authorization-code access token with audience `nvbes-account-service`.",
            )),
        );
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "nvbes Account Service",
        version = "0.1.0",
        description = "Independent OAuth 2.0 Resource Server for Account-owned data",
        contact(name = "nvbes", url = "https://nvbes.fr"),
        license(name = "AGPL-3.0-only"),
    ),
    tags(
        (name = "profile", description = "Account profile and avatar"),
        (name = "settings", description = "Account display and notification preferences"),
        (name = "privacy", description = "Consents, GPC and Account data export"),
        (name = "closure", description = "Coordinated Account closure saga"),
    ),
    paths(
        crate::profile_routes::get_profile,
        crate::profile_routes::update_profile,
        crate::avatar_routes::prepare_avatar_upload,
        crate::avatar_routes::download_avatar,
        crate::avatar_routes::delete_avatar,
        crate::preferences_routes::get_preferences,
        crate::preferences_routes::update_preferences,
        crate::notifications_routes::get_notifications,
        crate::notifications_routes::update_notifications,
        crate::consents_routes::list_consents,
        crate::consents_routes::grant_consent,
        crate::consents_routes::revoke_consent,
        crate::privacy_routes::get_gpc_status,
        crate::privacy_routes::request_export,
        crate::privacy_routes::download_export,
        crate::closure_routes::request_closure,
        crate::closure_routes::get_closure,
        crate::sessions_routes::list_sessions,
        crate::sessions_routes::revoke_session,
    ),
    modifiers(&AccountOAuthSecurity),
)]
pub struct AccountApiDoc;

pub async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(AccountApiDoc::openapi())
}

#[cfg(test)]
mod tests {
    use utoipa::OpenApi;

    use super::AccountApiDoc;

    #[test]
    fn contract_contains_only_canonical_account_routes() {
        let document = serde_json::to_value(AccountApiDoc::openapi()).expect("OpenAPI JSON");
        let paths = document["paths"].as_object().expect("OpenAPI paths");
        let expected = [
            "/api/v1/closure",
            "/api/v1/consents",
            "/api/v1/notifications",
            "/api/v1/preferences",
            "/api/v1/privacy/export",
            "/api/v1/privacy/gpc",
            "/api/v1/profile",
            "/api/v1/profile/avatar",
            "/api/v1/security/sessions",
            "/api/v1/security/sessions/{sessionId}",
        ];
        assert_eq!(paths.len(), expected.len());
        assert!(expected.iter().all(|path| paths.contains_key(*path)));
        assert!(paths.keys().all(|path| !path.contains("/auth")));
        assert!(paths.keys().all(|path| !path.contains("/legal")));
    }
}
