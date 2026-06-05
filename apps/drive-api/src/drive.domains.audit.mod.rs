#[path = "drive.domains.audit.models.rs"]
pub mod models;
#[path = "drive.domains.audit.routes.rs"]
mod routes;
#[path = "drive.domains.audit.service.rs"]
mod service;

pub use routes::router;
pub use service::*;
