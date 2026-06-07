#[path = "s3.client.rs"]
mod client;
#[path = "s3.multipart.rs"]
mod multipart;
#[path = "s3.objects.rs"]
mod objects;
#[path = "s3.presign.rs"]
mod presign;

use async_trait::async_trait;
use aws_sdk_s3::Client as S3Client;

use crate::error::StorageError;
use crate::trait_def::{
    ByteRange, CompletedUploadPart, ObjectMeta, ObjectStore, PresignedUrl, UploadedPart,
};

macro_rules! traced_s3_request {
    ($request:expr) => {{
        $request.customize().mutate_request(|request| {
            let traceparent = nvbes_core::trace_context::new_traceparent(true);
            request.headers_mut().insert(
                nvbes_core::trace_context::TRACEPARENT_HEADER,
                traceparent.to_header_value(),
            );
            request.headers_mut().insert(
                nvbes_core::trace_context::SENTRY_TRACE_HEADER,
                traceparent.to_sentry_trace_header_value(),
            );
        })
    }};
}

pub(crate) use traced_s3_request;

#[derive(Clone)]
pub struct S3ObjectStore {
    client: S3Client,
    bucket: String,
}

impl S3ObjectStore {
    pub async fn new(
        bucket: String,
        endpoint: &str,
        region: &str,
        access_key: &str,
        secret_key: &str,
    ) -> Self {
        client::new_client(bucket, endpoint, region, access_key, secret_key)
    }
}

#[async_trait]
impl ObjectStore for S3ObjectStore {
    async fn presign_upload(
        &self,
        key: &str,
        content_type: Option<&str>,
        expected_size_bytes: i64,
        expires: std::time::Duration,
    ) -> Result<PresignedUrl, StorageError> {
        presign::presign_upload(self, key, content_type, expected_size_bytes, expires).await
    }

    async fn presign_download(
        &self,
        key: &str,
        expires: std::time::Duration,
    ) -> Result<PresignedUrl, StorageError> {
        presign::presign_download(self, key, expires).await
    }

    async fn get_object(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        objects::get_object(self, key).await
    }

    async fn get_object_range(&self, key: &str, range: ByteRange) -> Result<Vec<u8>, StorageError> {
        objects::get_object_range(self, key, range).await
    }

    async fn create_multipart_upload(
        &self,
        key: &str,
        content_type: Option<&str>,
    ) -> Result<String, StorageError> {
        multipart::create_multipart_upload(self, key, content_type).await
    }

    async fn upload_part(
        &self,
        key: &str,
        multipart_upload_id: &str,
        part_number: i32,
        body: Vec<u8>,
    ) -> Result<UploadedPart, StorageError> {
        multipart::upload_part(self, key, multipart_upload_id, part_number, body).await
    }

    async fn complete_multipart_upload(
        &self,
        key: &str,
        multipart_upload_id: &str,
        parts: &[CompletedUploadPart],
    ) -> Result<(), StorageError> {
        multipart::complete_multipart_upload(self, key, multipart_upload_id, parts).await
    }

    async fn abort_multipart_upload(
        &self,
        key: &str,
        multipart_upload_id: &str,
    ) -> Result<(), StorageError> {
        multipart::abort_multipart_upload(self, key, multipart_upload_id).await
    }

    async fn put_object(
        &self,
        key: &str,
        content_type: Option<&str>,
        body: Vec<u8>,
    ) -> Result<(), StorageError> {
        objects::put_object(self, key, content_type, body).await
    }

    async fn delete_objects(&self, keys: &[String]) -> Result<(), StorageError> {
        objects::delete_objects(self, keys).await
    }

    async fn head_object(&self, key: &str) -> Result<ObjectMeta, StorageError> {
        objects::head_object(self, key).await
    }
}
