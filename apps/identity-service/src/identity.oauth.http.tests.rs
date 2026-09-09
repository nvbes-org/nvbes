use super::{authorization_router, router};
use crate::tokens::TokenService;
use crate::{browser::BrowserSecurity, mfa_crypto::MfaCrypto};

#[test]
fn http_module_is_kept_separate_from_mutating_authorization_routes() {
    let _router_type: fn(&str, std::sync::Arc<TokenService>) -> _ = router;
}

#[test]
fn authorization_router_requires_browser_and_mfa_runtime_state() {
    let _router_type: fn(
        sqlx::PgPool,
        std::sync::Arc<crate::oauth::clients::ClientRegistry>,
        BrowserSecurity,
        std::sync::Arc<MfaCrypto>,
    ) -> _ = authorization_router;
}
