#[path = "account.app.rs"]
pub mod app;
#[path = "account.auth.rs"]
pub mod auth;
#[path = "account.avatar.db.rs"]
mod avatar_db;
#[path = "account.avatar.models.rs"]
pub mod avatar_models;
#[path = "account.avatar.routes.rs"]
pub mod avatar_routes;
#[path = "account.closure.db.rs"]
mod closure_db;
#[path = "account.closure.routes.rs"]
pub mod closure_routes;
#[path = "account.config.rs"]
pub mod config;
#[path = "account.consents.db.rs"]
mod consents_db;
#[path = "account.consents.models.rs"]
pub mod consents_models;
#[path = "account.consents.routes.rs"]
pub mod consents_routes;
#[path = "account.error.rs"]
pub mod error;
#[path = "account.health.rs"]
pub mod health;
#[path = "account.http.rs"]
pub mod http;
#[path = "account.notifications.db.rs"]
mod notifications_db;
#[path = "account.notifications.routes.rs"]
pub mod notifications_routes;
#[path = "account.openapi.rs"]
pub mod openapi;
#[path = "account.preferences.db.rs"]
mod preferences_db;
#[path = "account.preferences.routes.rs"]
pub mod preferences_routes;
#[path = "account.privacy.db.rs"]
mod privacy_db;
#[path = "account.privacy.models.rs"]
pub mod privacy_models;
#[path = "account.privacy.routes.rs"]
pub mod privacy_routes;
#[path = "account.profile.db.rs"]
mod profile_db;
#[path = "account.profile.models.rs"]
pub mod profile_models;
#[path = "account.profile.routes.rs"]
pub mod profile_routes;
#[path = "account.profile.validation.rs"]
mod profile_validation;
#[path = "account.settings.models.rs"]
pub mod settings_models;

#[cfg(test)]
#[path = "account.boundary.tests.rs"]
mod boundary_tests;
