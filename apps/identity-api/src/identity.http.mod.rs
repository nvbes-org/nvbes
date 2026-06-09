#[path = "identity.http.cookies.rs"]
pub mod cookies;
#[path = "identity.http.cors.rs"]
pub mod cors;
#[path = "identity.http.error.rs"]
pub mod error;
#[path = "identity.http.middleware.mod.rs"]
pub mod middleware;
#[path = "identity.http.request.rs"]
pub mod request;
pub use nvbes_core::security::security_headers;

#[path = "identity.http.openapi.rs"]
pub mod openapi;

#[path = "identity.http.observability.rs"]
pub mod observability;

#[path = "identity.http.routes.csp_report.rs"]
pub mod csp_report;
#[path = "identity.http.routes.well_known.rs"]
pub mod well_known;

#[path = "identity.http.routes.rs"]
pub mod routes;

pub use routes::router;
