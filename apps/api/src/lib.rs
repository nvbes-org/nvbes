//! NVBES Modular Monolith Serverless API.

#[path = "api.config.rs"]
pub mod config;
#[path = "api.health.rs"]
pub mod health;
#[path = "api.identity.rs"]
pub mod identity;
#[path = "api.migrations.rs"]
pub mod migrations;
#[path = "api.openapi.rs"]
pub mod openapi;
#[path = "api.routes.rs"]
pub mod routes;
#[path = "api.server.rs"]
pub mod server;

pub use config::ApiConfig;
pub use openapi::openapi_spec;
pub use routes::build_router;
pub use server::run_server;

#[cfg(test)]
#[path = "api.tests.rs"]
mod tests;
