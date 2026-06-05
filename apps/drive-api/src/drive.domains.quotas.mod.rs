#[path = "drive.domains.quotas.db.rs"]
pub mod db;
#[path = "drive.domains.quotas.logic.rs"]
pub mod logic;
#[path = "drive.domains.quotas.models.rs"]
pub mod models;
#[path = "drive.domains.quotas.observability.rs"]
pub mod observability;
#[path = "drive.domains.quotas.routes.rs"]
mod routes;
#[path = "drive.domains.quotas.service.rs"]
pub mod service;
#[path = "drive.domains.quotas.types.rs"]
pub mod types;

pub use routes::router;
pub use service::*;
pub use types::{
    BandwidthOutUsageInput, FileUploadedUsageInput, QuotaResponse, StorageReleasedUsageInput,
};
