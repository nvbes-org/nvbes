use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[path = "drive.domains.public_api.routes.v1_handlers.files.browse.rs"]
mod browse;
#[path = "drive.domains.public_api.routes.v1_handlers.files.download.rs"]
mod download;
#[path = "drive.domains.public_api.routes.v1_handlers.files.mutations.rs"]
mod mutations;

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct ListObjectsQuery {
    pub parent_id: Option<Uuid>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct CreateFolderRequest {
    pub parent_id: Option<Uuid>,
    pub name: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct RenameObjectRequest {
    pub name: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct MoveObjectRequest {
    pub destination_parent_id: Option<Uuid>,
}

pub use browse::{__path_create_folder, __path_list_objects, create_folder, list_objects};
pub use download::{__path_create_download_url, create_download_url};
pub use mutations::{
    __path_move_object, __path_rename_object, __path_trash_object, move_object, rename_object,
    trash_object,
};
