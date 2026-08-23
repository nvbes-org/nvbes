#[path = "drive.http.account_closure.rs"]
pub mod account_closure;
#[path = "drive.http.account_export.rs"]
pub mod account_export;
#[path = "drive.http.error.rs"]
pub mod error;
#[path = "drive.http.middleware.100_continue.rs"]
pub mod expect_continue;
#[path = "drive.http.observability.rs"]
pub mod observability;
#[path = "drive.http.openapi.rs"]
pub mod openapi;
#[path = "drive.http.request.rs"]
pub mod request;
pub use nvbes_core::security::security_headers;

#[path = "drive.http.routes.rs"]
pub mod routes;

pub use routes::router;
