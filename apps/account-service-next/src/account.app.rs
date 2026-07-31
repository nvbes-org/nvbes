use std::sync::Arc;

use axum::{
    Router,
    http::{
        HeaderValue, Method,
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    },
};
use nvbes_identity_sdk::IdentityJwtVerifier;
use nvbes_storage::{MockObjectStore, ObjectStore, S3ObjectStore};
use sqlx::PgPool;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    auth::ACCOUNT_AUDIENCE,
    config::{AccountConfig, AvatarStorageConfig},
};

#[derive(Clone)]
pub struct AppState {
    pub config: AccountConfig,
    pub db: PgPool,
    pub jwt_verifier: IdentityJwtVerifier,
    pub avatar_storage: Arc<dyn ObjectStore>,
}

impl AppState {
    pub async fn bootstrap(config: AccountConfig, db: PgPool) -> anyhow::Result<Self> {
        let jwt_verifier =
            IdentityJwtVerifier::new(&config.identity_service_base_url, ACCOUNT_AUDIENCE)?;
        let avatar_storage = build_avatar_storage(&config.avatar_storage).await;
        Ok(Self {
            config,
            db,
            jwt_verifier,
            avatar_storage,
        })
    }
}

pub fn build_router(state: AppState) -> Router {
    let allowed_origin = HeaderValue::from_str(&state.config.account_web_origin)
        .expect("validated Account web origin is a valid header");
    let cors = CorsLayer::new()
        .allow_origin(allowed_origin)
        .allow_methods([
            Method::GET,
            Method::PATCH,
            Method::POST,
            Method::PUT,
            Method::DELETE,
        ])
        .allow_headers([ACCEPT, AUTHORIZATION, CONTENT_TYPE]);

    crate::http::router(&state)
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}

async fn build_avatar_storage(config: &AvatarStorageConfig) -> Arc<dyn ObjectStore> {
    match config {
        AvatarStorageConfig::Mock => Arc::new(MockObjectStore::new()),
        AvatarStorageConfig::S3 {
            bucket,
            endpoint,
            public_endpoint,
            region,
            access_key,
            secret_key,
        } => Arc::new(
            S3ObjectStore::new(
                bucket.clone(),
                endpoint,
                public_endpoint.as_deref(),
                region,
                access_key,
                secret_key,
            )
            .await,
        ),
    }
}
