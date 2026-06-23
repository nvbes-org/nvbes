#[path = "drive.domains.files.core.rs"]
pub mod core;
#[path = "drive.domains.files.db.rs"]
pub mod db;
#[path = "drive.domains.files.db.lifecycle.rs"]
pub mod db_lifecycle;
#[path = "drive.domains.files.etag.rs"]
pub mod etag;
#[path = "drive.domains.files.lifecycle.rs"]
pub mod lifecycle;
#[path = "drive.domains.files.models.rs"]
pub mod models;
#[path = "drive.domains.files.db.queries.rs"]
pub mod queries;
#[path = "drive.domains.files.routes.rs"]
pub mod routes;
#[path = "drive.domains.files.service.rs"]
pub mod service;
#[path = "drive.domains.files.transfer.rs"]
pub mod transfer;
#[path = "drive.domains.files.types.rs"]
pub mod types;

pub use routes::router;
pub use service::{
    create_download_url, create_folder, delete_object, download_object, list_objects, list_trash,
    move_object, rename_object, restore_object, trash_object,
};
pub use types::{
    CreateFolderInput, DownloadUrlResponse, ListObjectsInput, ListObjectsResponse, MoveObjectInput,
    ObjectResponse, RenameObjectInput,
};
