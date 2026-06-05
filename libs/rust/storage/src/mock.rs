use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, convert::TryFrom};

use async_trait::async_trait;
use tokio::sync::Mutex;
use tracing::info;

use crate::error::StorageError;
use crate::trait_def::{
    ByteRange, CompletedUploadPart, ObjectMeta, ObjectStore, PresignedUrl, UploadedPart,
};

pub struct MockObjectStore {
    deleted: Arc<Mutex<Vec<String>>>,
    object_sizes: Arc<Mutex<HashMap<String, i64>>>,
    multipart_parts: Arc<Mutex<HashMap<String, Vec<UploadedPart>>>>,
}

impl MockObjectStore {
    pub fn new() -> Self {
        Self {
            deleted: Arc::new(Mutex::new(Vec::new())),
            object_sizes: Arc::new(Mutex::new(HashMap::new())),
            multipart_parts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn deleted_keys(&self) -> Vec<String> {
        self.deleted.lock().await.clone()
    }
}

impl Default for MockObjectStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ObjectStore for MockObjectStore {
    async fn presign_upload(
        &self,
        key: &str,
        _content_type: Option<&str>,
        expected_size_bytes: i64,
        expires: Duration,
    ) -> Result<PresignedUrl, StorageError> {
        info!(key, "MockStorage: presign upload (not real)");
        self.object_sizes
            .lock()
            .await
            .insert(key.to_string(), expected_size_bytes);
        Ok(PresignedUrl {
            url: format!("http://localhost:9000/mock/upload/{key}"),
            method: "PUT",
            expires_in: expires,
        })
    }

    async fn presign_download(
        &self,
        key: &str,
        expires: Duration,
    ) -> Result<PresignedUrl, StorageError> {
        info!(key, "MockStorage: presign download (not real)");
        Ok(PresignedUrl {
            url: format!("http://localhost:9000/mock/download/{key}"),
            method: "GET",
            expires_in: expires,
        })
    }

    async fn get_object(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        info!(key, "MockStorage: get_object (empty)");
        let size = self
            .object_sizes
            .lock()
            .await
            .get(key)
            .copied()
            .unwrap_or_default();
        let size = usize::try_from(size).unwrap_or_default();
        Ok(vec![0; size])
    }

    async fn get_object_range(&self, key: &str, range: ByteRange) -> Result<Vec<u8>, StorageError> {
        info!(key, "MockStorage: get_object_range (empty)");
        let object = self.get_object(key).await?;
        let start = usize::try_from(range.start)
            .map_err(|e| StorageError::Other(format!("invalid range start: {e}")))?;
        let end_inclusive = usize::try_from(range.end_inclusive)
            .map_err(|e| StorageError::Other(format!("invalid range end: {e}")))?;

        if start >= object.len() || end_inclusive < start {
            return Ok(Vec::new());
        }

        let end_exclusive = end_inclusive.saturating_add(1).min(object.len());
        Ok(object[start..end_exclusive].to_vec())
    }

    async fn create_multipart_upload(
        &self,
        key: &str,
        _content_type: Option<&str>,
    ) -> Result<String, StorageError> {
        let multipart_upload_id = format!("mock-multipart-{key}");
        self.multipart_parts
            .lock()
            .await
            .insert(multipart_upload_id.clone(), Vec::new());
        Ok(multipart_upload_id)
    }

    async fn upload_part(
        &self,
        _key: &str,
        multipart_upload_id: &str,
        part_number: i32,
        body: Vec<u8>,
    ) -> Result<UploadedPart, StorageError> {
        let size_bytes = i64::try_from(body.len())
            .map_err(|e| StorageError::Other(format!("invalid part size: {e}")))?;
        let uploaded = UploadedPart {
            part_number,
            etag: format!("\"mock-part-{part_number}\""),
            size_bytes,
        };
        self.multipart_parts
            .lock()
            .await
            .entry(multipart_upload_id.to_owned())
            .or_default()
            .push(uploaded.clone());
        Ok(uploaded)
    }

    async fn complete_multipart_upload(
        &self,
        key: &str,
        multipart_upload_id: &str,
        parts: &[CompletedUploadPart],
    ) -> Result<(), StorageError> {
        let uploaded_parts = self.multipart_parts.lock().await;
        let size_bytes = uploaded_parts
            .get(multipart_upload_id)
            .map(|parts| parts.iter().map(|part| part.size_bytes).sum())
            .unwrap_or_default();
        self.object_sizes
            .lock()
            .await
            .insert(key.to_owned(), size_bytes);
        let _ = parts;
        Ok(())
    }

    async fn abort_multipart_upload(
        &self,
        _key: &str,
        multipart_upload_id: &str,
    ) -> Result<(), StorageError> {
        self.multipart_parts
            .lock()
            .await
            .remove(multipart_upload_id);
        Ok(())
    }

    async fn put_object(
        &self,
        key: &str,
        _content_type: Option<&str>,
        body: Vec<u8>,
    ) -> Result<(), StorageError> {
        let size_bytes = i64::try_from(body.len())
            .map_err(|e| StorageError::Other(format!("invalid body size: {e}")))?;
        self.object_sizes
            .lock()
            .await
            .insert(key.to_owned(), size_bytes);
        Ok(())
    }

    async fn delete_objects(&self, keys: &[String]) -> Result<(), StorageError> {
        info!(count = keys.len(), "MockStorage: delete objects (no-op)");
        self.deleted.lock().await.extend(keys.iter().cloned());
        Ok(())
    }

    async fn head_object(&self, key: &str) -> Result<ObjectMeta, StorageError> {
        let size_bytes = self
            .object_sizes
            .lock()
            .await
            .get(key)
            .copied()
            .unwrap_or_default();
        Ok(ObjectMeta {
            size_bytes,
            etag: Some("\"mock-etag\"".to_string()),
            content_encoding: None,
        })
    }
}
