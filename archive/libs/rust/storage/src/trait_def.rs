use std::time::Duration;

use async_trait::async_trait;

use crate::error::StorageError;

#[derive(Debug, Clone)]
pub struct PresignedUrl {
    pub url: String,
    pub method: &'static str,
    pub expires_in: Duration,
}

#[derive(Debug, Clone)]
pub struct CompletedUploadPart {
    pub part_number: i32,
    pub etag: String,
}

#[derive(Debug, Clone)]
pub struct UploadedPart {
    pub part_number: i32,
    pub etag: String,
    pub size_bytes: i64,
}

#[derive(Debug, Clone, Copy)]
pub struct ByteRange {
    pub start: i64,
    pub end_inclusive: i64,
}

#[async_trait]
pub trait ObjectStore: Send + Sync {
    async fn presign_upload(
        &self,
        key: &str,
        content_type: Option<&str>,
        expected_size_bytes: i64,
        expires: Duration,
    ) -> Result<PresignedUrl, StorageError>;

    async fn presign_download(
        &self,
        key: &str,
        expires: Duration,
    ) -> Result<PresignedUrl, StorageError>;

    async fn get_object(&self, key: &str) -> Result<Vec<u8>, StorageError>;

    async fn get_object_range(&self, key: &str, range: ByteRange) -> Result<Vec<u8>, StorageError>;

    async fn create_multipart_upload(
        &self,
        key: &str,
        content_type: Option<&str>,
    ) -> Result<String, StorageError>;

    async fn upload_part(
        &self,
        key: &str,
        multipart_upload_id: &str,
        part_number: i32,
        body: Vec<u8>,
    ) -> Result<UploadedPart, StorageError>;

    async fn complete_multipart_upload(
        &self,
        key: &str,
        multipart_upload_id: &str,
        parts: &[CompletedUploadPart],
    ) -> Result<(), StorageError>;

    async fn abort_multipart_upload(
        &self,
        key: &str,
        multipart_upload_id: &str,
    ) -> Result<(), StorageError>;

    async fn put_object(
        &self,
        key: &str,
        content_type: Option<&str>,
        body: Vec<u8>,
    ) -> Result<(), StorageError>;

    async fn delete_objects(&self, keys: &[String]) -> Result<(), StorageError>;

    async fn head_object(&self, key: &str) -> Result<ObjectMeta, StorageError>;
}

#[derive(Debug, Clone)]
pub struct ObjectMeta {
    pub size_bytes: i64,
    pub etag: Option<String>,
    pub content_encoding: Option<String>,
}
