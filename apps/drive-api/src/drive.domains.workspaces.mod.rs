#[path = "drive.domains.workspaces.db.rs"]
pub mod db;
#[path = "drive.domains.workspaces.lifecycle.rs"]
pub mod lifecycle;
#[path = "drive.domains.workspaces.models.rs"]
pub mod models;
#[path = "drive.domains.workspaces.routes.rs"]
mod routes;
#[path = "drive.domains.workspaces.service.rs"]
mod service;
#[path = "drive.domains.workspaces.types.rs"]
pub mod types;
#[path = "drive.domains.workspaces.validation.rs"]
pub mod validation;

pub use routes::router;
pub use service::{create_workspace, get_workspace, list_workspaces, update_workspace};
