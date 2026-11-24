use axum::Router;
use sqlx::PgPool;

use nvbes_identity_service::{
    oauth::http::{
        authorization_router, introspection_router, passkey_login_router, recovery_router,
        router_with_logout, rp_logout_router, token_router, webauthn_router,
    },
    session_context::{self, SessionContextGuard},
};

use crate::identity_runtime::IdentityRuntime;

/// Mounts every Identity HTTP surface. The runtime is already validated, so
/// there is no partially mounted fallback: either Identity serves fully or
/// the process never started. Every route checks the environment of the
/// browser session it is presented with.
pub fn build_router(pool: PgPool, identity: &IdentityRuntime) -> anyhow::Result<Router> {
    let session_context =
        SessionContextGuard::new(pool.clone(), identity.browser.clone(), &identity.limiter)
            .ok_or_else(|| anyhow::anyhow!("session context key derivation failed"))?;
    let issuer = identity.tokens.issuer().to_owned();
    let router = Router::new()
        .merge(router_with_logout(&issuer, identity.tokens.clone(), true))
        .merge(token_router(
            pool.clone(),
            identity.clients.clone(),
            identity.tokens.clone(),
            identity.limiter.clone(),
        ))
        .merge(rp_logout_router(
            pool.clone(),
            identity.clients.clone(),
            identity.tokens.clone(),
            identity.browser.clone(),
            identity.limiter.clone(),
        ))
        .merge(authorization_router(
            pool.clone(),
            identity.clients.clone(),
            identity.browser.clone(),
            identity.mfa.clone(),
            identity.limiter.clone(),
        ))
        .merge(passkey_login_router(
            pool.clone(),
            identity.clients.clone(),
            identity.browser.clone(),
            identity.webauthn.clone(),
            identity.limiter.clone(),
        ))
        .merge(recovery_router(
            pool.clone(),
            identity.browser.clone(),
            identity.webauthn.clone(),
            identity.limiter.clone(),
        ))
        .merge(webauthn_router(
            pool.clone(),
            identity.browser.clone(),
            identity.webauthn.clone(),
            identity.limiter.clone(),
        ));
    let router = match &identity.resources {
        Some(resources) => router.merge(introspection_router(
            pool,
            identity.clients.clone(),
            identity.tokens.clone(),
            resources.clone(),
            identity.limiter.clone(),
        )),
        None => router,
    };
    Ok(router.layer(axum::middleware::from_fn_with_state(
        session_context,
        session_context::enforce,
    )))
}
