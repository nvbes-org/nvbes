use super::models::StorageObjectRecord;
use crate::http::error::AppError;

pub fn ensure_object_etag(
    expected_etag: Option<&str>,
    object: &StorageObjectRecord,
) -> Result<(), AppError> {
    let Some(expected_etag) = expected_etag else {
        return Ok(());
    };

    let current_etag =
        nvbes_core::http::etag::resource_etag("storage_object", object.id, object.updated_at);
    if nvbes_core::http::etag::etag_list_matches(expected_etag, &current_etag) {
        return Ok(());
    }

    Err(AppError::precondition_failed(
        "etag_mismatch",
        "Object has changed since the client last read it.",
    ))
}
