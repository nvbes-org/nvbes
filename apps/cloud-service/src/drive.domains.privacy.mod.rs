#[path = "drive.domains.privacy.db.rs"]
pub mod db;
#[path = "drive.domains.privacy.routes.rs"]
mod routes;
#[path = "drive.domains.privacy.service.rs"]
mod service;
#[path = "drive.domains.privacy.types.rs"]
pub mod types;

pub use routes::router;
pub use service::{get_request, request_workspace_delete, request_workspace_export};
