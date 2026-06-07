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
pub use service::{
    ensure_upload_allowed_tx, get_quota, record_bandwidth_out_tx, record_file_uploaded_tx,
    release_storage_tx,
};
pub use types::{
    BandwidthOutUsageInput, FileUploadedUsageInput, QuotaResponse, StorageReleasedUsageInput,
};
