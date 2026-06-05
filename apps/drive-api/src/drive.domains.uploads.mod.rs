#[path = "drive.domains.uploads.core.rs"]
pub mod core;
#[path = "drive.domains.uploads.db.rs"]
pub mod db;
#[path = "drive.domains.uploads.lifecycle.rs"]
pub mod lifecycle;
#[path = "drive.domains.uploads.logic.rs"]
pub mod logic;
#[path = "drive.domains.uploads.models.rs"]
pub mod models;
#[path = "drive.domains.uploads.db.queries.rs"]
pub mod queries;
#[path = "drive.domains.uploads.routes.rs"]
mod routes;
#[path = "drive.domains.uploads.scan.rs"]
pub mod scan;
#[path = "drive.domains.uploads.service.rs"]
pub mod service;
#[path = "drive.domains.uploads.types.rs"]
pub mod types;

pub use routes::router;
pub use service::*;
