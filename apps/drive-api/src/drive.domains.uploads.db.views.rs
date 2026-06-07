use crate::domains::{
    files::models::{StorageObjectStatus, StorageObjectType},
    uploads::{models::StorageObjectRecord, types::UploadObjectView},
};

pub fn storage_object_type_as_str(object_type: StorageObjectType) -> &'static str {
    match object_type {
        StorageObjectType::File => "file",
        StorageObjectType::Folder => "folder",
    }
}

pub fn storage_object_status_as_str(status: StorageObjectStatus) -> &'static str {
    match status {
        StorageObjectStatus::Pending => "pending",
        StorageObjectStatus::Active => "active",
        StorageObjectStatus::Trashed => "trashed",
        StorageObjectStatus::Deleted => "deleted",
        StorageObjectStatus::Quarantined => "quarantined",
    }
}

pub fn map_record_to_view(record: StorageObjectRecord) -> UploadObjectView {
    UploadObjectView {
        id: record.id,
        workspace_id: record.workspace_id,
        parent_id: record.parent_id,
        name: record.name,
        object_type: storage_object_type_as_str(record.object_type).to_owned(),
        status: storage_object_status_as_str(record.status).to_owned(),
        scan_status: record.scan_status,
        size_bytes: record.size_bytes,
        mime_type: record.mime_type,
        checksum: record.checksum,
        created_by: record.created_by,
        created_by_principal_id: record.created_by_principal_id,
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}
