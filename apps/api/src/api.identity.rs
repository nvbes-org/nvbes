use std::sync::Arc;

use axum::Router;
use sqlx::PgPool;

use nvbes_identity_service::{
    browser::BrowserSecurity,
    mfa_crypto::MfaCrypto,
    oauth::{
        clients::ClientRegistry,
        http::{
            authorization_router, introspection_router, passkey_login_router, recovery_router,
            router_with_logout, rp_logout_router, token_router, webauthn_router,
        },
        resources::ResourceServers,
    },
    rate_limits::RateLimiter,
    tokens::TokenService,
    tokens_config::TokenConfig,
    webauthn,
};

use crate::config::ApiConfig;

pub fn build_router(pool: PgPool, config: &ApiConfig) -> (Router, Option<Arc<TokenService>>) {
    let is_dev = matches!(config.environment.as_str(), "development" | "test");

    let token_service = match init_token_service(config, is_dev) {
        Ok(svc) => Some(Arc::new(svc)),
        Err(e) => {
            tracing::warn!("TokenService not initialized: {e}");
            None
        }
    };

    let Some(tokens) = token_service.clone() else {
        return (Router::new(), None);
    };

    let issuer = tokens.issuer().to_owned();

    let clients = match init_clients(is_dev) {
        Ok(c) => Arc::new(c),
        Err(e) => {
            tracing::warn!("OAuth clients not initialized: {e}");
            let discovery = router_with_logout(&issuer, tokens.clone(), false);
            return (discovery, Some(tokens));
        }
    };

    let limiter = match init_limiter() {
        Ok(l) => l,
        Err(e) => {
            tracing::warn!("RateLimiter not initialized: {e}");
            let discovery = router_with_logout(&issuer, tokens.clone(), false);
            return (discovery, Some(tokens));
        }
    };

    let browser_origin = std::env::var("NVBES_IDENTITY_BROWSER_ORIGIN")
        .unwrap_or_else(|_| "http://localhost:5173".into());

    let browser = match BrowserSecurity::new(&browser_origin, is_dev) {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!("BrowserSecurity not initialized: {e}");
            let discovery = router_with_logout(&issuer, tokens.clone(), false);
            return (discovery, Some(tokens));
        }
    };

    let mfa = match init_mfa() {
        Ok(m) => Arc::new(m),
        Err(e) => {
            tracing::warn!("MfaCrypto not initialized: {e}");
            let discovery = router_with_logout(&issuer, tokens.clone(), false);
            return (discovery, Some(tokens));
        }
    };

    let webauthn_server = match init_webauthn(&browser_origin) {
        Ok(w) => Arc::new(w),
        Err(e) => {
            tracing::warn!("Webauthn not initialized: {e}");
            let discovery = router_with_logout(&issuer, tokens.clone(), false);
            return (discovery, Some(tokens));
        }
    };

    let mut router = Router::new()
        .merge(router_with_logout(&issuer, tokens.clone(), true))
        .merge(token_router(
            pool.clone(),
            clients.clone(),
            tokens.clone(),
            limiter.clone(),
        ))
        .merge(rp_logout_router(
            pool.clone(),
            clients.clone(),
            tokens.clone(),
            browser.clone(),
            limiter.clone(),
        ))
        .merge(authorization_router(
            pool.clone(),
            clients.clone(),
            browser.clone(),
            mfa,
            limiter.clone(),
        ))
        .merge(passkey_login_router(
            pool.clone(),
            clients.clone(),
            browser.clone(),
            webauthn_server.clone(),
            limiter.clone(),
        ))
        .merge(recovery_router(
            pool.clone(),
            browser.clone(),
            webauthn_server.clone(),
            limiter.clone(),
        ))
        .merge(webauthn_router(
            pool.clone(),
            browser,
            webauthn_server,
            limiter.clone(),
        ));

    let resources = std::env::var("NVBES_IDENTITY_RESOURCE_SERVERS_JSON")
        .ok()
        .and_then(|json| ResourceServers::from_json(&json).ok());
    if let Some(resources) = resources {
        router = router.merge(introspection_router(
            pool,
            clients,
            tokens.clone(),
            Arc::new(resources),
            limiter,
        ));
    }

    (router, Some(tokens))
}

fn init_token_service(config: &ApiConfig, is_dev: bool) -> Result<TokenService, anyhow::Error> {
    match TokenConfig::from_env(&config.environment) {
        Ok(cfg) => TokenService::new(cfg).map_err(|e| anyhow::anyhow!("{e}")),
        Err(e) if is_dev => {
            tracing::info!("generating dev token signing keys: {e}");
            let dev_cfg = dev_token_config()?;
            TokenService::new(dev_cfg).map_err(|err| anyhow::anyhow!("{err}"))
        }
        Err(e) => Err(anyhow::anyhow!("TokenConfig::from_env failed: {e}")),
    }
}

fn dev_token_config() -> Result<TokenConfig, anyhow::Error> {
    let private = rsa::RsaPrivateKey::new(&mut rsa::rand_core::OsRng, 2048)
        .map_err(|e| anyhow::anyhow!("RSA gen error: {e}"))?;
    let priv_pem = rsa::pkcs8::EncodePrivateKey::to_pkcs8_pem(&private, Default::default())
        .map_err(|e| anyhow::anyhow!("RSA private pem error: {e}"))?
        .to_string();
    let pub_pem = rsa::pkcs8::EncodePublicKey::to_public_key_pem(
        &private.to_public_key(),
        Default::default(),
    )
    .map_err(|e| anyhow::anyhow!("RSA public pem error: {e}"))?;

    TokenConfig::from_values(
        "test",
        "http://localhost:8080".into(),
        "nvbes-dev-key-1".into(),
        priv_pem,
        pub_pem,
        "nvbes-account-service,nvbes-billing-service".into(),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
}

fn init_clients(is_dev: bool) -> Result<ClientRegistry, anyhow::Error> {
    let registry_json = std::env::var("NVBES_IDENTITY_OAUTH_CLIENTS_JSON").unwrap_or_else(|_| {
        r#"[
            {
                "client_id": "nvbes-web",
                "display_name": "NVBES Web",
                "redirect_uris": ["http://localhost:5173/auth/callback", "http://localhost:3000/auth/callback"],
                "post_logout_redirect_uris": ["http://localhost:5173", "http://localhost:3000"],
                "resources": {
                    "http://localhost:8080/account": {
                        "audience": "nvbes-account-service",
                        "scopes": ["account:read", "account:write"]
                    },
                    "http://localhost:8080/billing": {
                        "audience": "nvbes-billing-service",
                        "scopes": ["billing:read", "billing:checkout"]
                    }
                },
                "allow_refresh": true,
                "require_dpop": false,
                "minimum_authentication": "primary"
            }
        ]"#.into()
    });

    ClientRegistry::from_json(&registry_json, is_dev).map_err(|e| anyhow::anyhow!("{e}"))
}

fn init_limiter() -> Result<RateLimiter, anyhow::Error> {
    let key_b64 = std::env::var("NVBES_IDENTITY_RATE_LIMIT_KEY")
        .unwrap_or_else(|_| "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=".into());
    RateLimiter::from_base64(&key_b64).map_err(|e| anyhow::anyhow!("{e}"))
}

fn init_mfa() -> Result<MfaCrypto, anyhow::Error> {
    let key = [0u8; 32];
    MfaCrypto::with_rotation(1, key, None).map_err(|e| anyhow::anyhow!("{e}"))
}

fn init_webauthn(origin: &str) -> Result<webauthn_rs::Webauthn, anyhow::Error> {
    let rp_origin = reqwest::Url::parse(origin)?;
    let rp_id = rp_origin
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("missing WebAuthn RP hostname"))?;
    webauthn::build_server(rp_id, origin).map_err(|e| anyhow::anyhow!("{e}"))
}
