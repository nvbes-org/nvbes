use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, ToSchema)]
pub(super) struct CreateUploadRequest {
    #[serde(alias = "parentId")]
    pub parent_id: Option<Uuid>,
    pub name: String,
    #[serde(alias = "mimeType")]
    pub mime_type: String,
    #[serde(alias = "expectedSizeBytes")]
    pub expected_size_bytes: i64,
    #[serde(alias = "expectedChecksum")]
    pub expected_checksum: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub(super) struct CompleteUploadRequest {
    #[serde(alias = "sizeBytes")]
    pub size_bytes: i64,
    pub checksum: Option<String>,
}
