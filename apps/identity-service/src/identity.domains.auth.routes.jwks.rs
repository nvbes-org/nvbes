use axum::{Router, body::Body, http::Request, middleware::Next, routing::get};

use super::jwks::get_jwks;
use crate::app::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/.well-known/jwks.json", get(get_jwks))
        .layer(axum::middleware::from_fn(
            |req: Request<Body>, next: Next| async {
                let mut res = next.run(req).await;
                nvbes_core::security::insert_cdn_cache_headers(res.headers_mut(), 3600, 86400);
                res
            },
        ))
}
