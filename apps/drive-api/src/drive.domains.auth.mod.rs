#[path = "drive.domains.auth.core.rs"]
pub mod core;
#[path = "drive.domains.auth.identity.rs"]
pub mod identity;
#[path = "drive.domains.auth.identity_sync.rs"]
pub mod identity_sync;
#[path = "drive.domains.auth.jwt.rs"]
pub mod jwt;
#[path = "drive.domains.auth.security.rs"]
pub mod security;

#[path = "drive.domains.auth.current.rs"]
pub mod current;
#[path = "drive.domains.auth.service.rs"]
pub mod service;
#[path = "drive.domains.auth.types.rs"]
pub mod types;

#[cfg(test)]
#[path = "drive.domains.auth.e2e.tests.rs"]
mod e2e_tests;

pub use service::*;

use axum::Router;

pub fn router(state: &crate::app::AppState) -> Router<crate::app::AppState> {
    current::router(state)
}
