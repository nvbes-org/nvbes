#[path = "drive.domains.public_api.routes.v1_handlers.files.rs"]
pub mod files;
#[path = "drive.domains.public_api.routes.v1_handlers.me.rs"]
pub mod me;
#[path = "drive.domains.public_api.routes.v1_handlers.quotas_audit.rs"]
pub mod quotas_audit;
#[path = "drive.domains.public_api.routes.v1_handlers.share_links.rs"]
pub mod share_links;
#[path = "drive.domains.public_api.routes.v1_handlers.uploads.rs"]
pub mod uploads;

pub use files::{
    __path_create_download_url, __path_create_folder, __path_list_objects, __path_move_object,
    __path_rename_object, __path_trash_object, create_download_url, create_folder, list_objects,
    move_object, rename_object, trash_object,
};
pub use me::{__path_list_workspaces, __path_me, list_workspaces, me};
pub use quotas_audit::{__path_get_quota, __path_list_audit_events, get_quota, list_audit_events};
pub use share_links::{
    __path_create_share_link, __path_list_share_links, __path_revoke_share_link,
    __path_update_share_link, create_share_link, list_share_links, revoke_share_link,
    update_share_link,
};
pub use uploads::{
    __path_cancel_upload, __path_complete_upload, __path_create_upload, cancel_upload,
    complete_upload, create_upload,
};
