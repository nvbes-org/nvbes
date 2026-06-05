use std::time::Duration;

use async_trait::async_trait;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart, Delete, ObjectIdentifier};
use tracing::debug;

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
        let credentials =
            aws_sdk_s3::config::Credentials::new(access_key, secret_key, None, None, "nvbes");

        let config = aws_sdk_s3::Config::builder()
            .region(aws_sdk_s3::config::Region::new(region.to_string()))
            .endpoint_url(endpoint)
            .credentials_provider(credentials)
            .behavior_version_latest()
            .force_path_style(true)
            .build();

        let client = S3Client::from_conf(config);

        Self { client, bucket }
    }
}

#[async_trait]
impl ObjectStore for S3ObjectStore {
    async fn presign_upload(
        &self,
        key: &str,
        content_type: Option<&str>,
        expected_size_bytes: i64,
        expires: Duration,
    ) -> Result<PresignedUrl, StorageError> {
        let presign_config = PresigningConfig::expires_in(expires)?;

        let mut req = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_length(expected_size_bytes);

        if let Some(ct) = content_type {
            req = req.content_type(ct);
        }
        req = req.content_encoding("identity");

        let presigned = req.presigned(presign_config).await?;

        debug!(key, bucket = %self.bucket, expires_secs = expires.as_secs(), "S3 presigned upload URL generated");

        Ok(PresignedUrl {
            url: presigned.uri().to_string(),
            method: "PUT",
            expires_in: expires,
        })
    }

    async fn presign_download(
        &self,
        key: &str,
        expires: Duration,
    ) -> Result<PresignedUrl, StorageError> {
        let presign_config = PresigningConfig::expires_in(expires)?;

        let presigned = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(presign_config)
            .await?;

        debug!(key, bucket = %self.bucket, expires_secs = expires.as_secs(), "S3 presigned download URL generated");

        Ok(PresignedUrl {
            url: presigned.uri().to_string(),
            method: "GET",
            expires_in: expires,
        })
    }

    async fn get_object(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let resp = traced_s3_request!(self.client.get_object().bucket(&self.bucket).key(key))
            .send()
            .await?;

        let data = resp
            .body
            .collect()
            .await
            .map_err(|e| StorageError::S3(format!("failed to read object body: {e}")))?;
        let bytes = data.into_bytes().to_vec();

        debug!(key, bucket = %self.bucket, size = bytes.len(), "S3 object downloaded");

        Ok(bytes)
    }

    async fn get_object_range(&self, key: &str, range: ByteRange) -> Result<Vec<u8>, StorageError> {
        let resp = traced_s3_request!(
            self.client
                .get_object()
                .bucket(&self.bucket)
                .key(key)
                .range(format!("bytes={}-{}", range.start, range.end_inclusive))
        )
        .send()
        .await?;

        let data = resp
            .body
            .collect()
            .await
            .map_err(|e| StorageError::S3(format!("failed to read object range body: {e}")))?;
        let bytes = data.into_bytes().to_vec();

        debug!(
            key,
            bucket = %self.bucket,
            start = range.start,
            end = range.end_inclusive,
            size = bytes.len(),
            "S3 object range downloaded"
        );

        Ok(bytes)
    }

    async fn create_multipart_upload(
        &self,
        key: &str,
        content_type: Option<&str>,
    ) -> Result<String, StorageError> {
        let mut req = self
            .client
            .create_multipart_upload()
            .bucket(&self.bucket)
            .key(key);

        if let Some(ct) = content_type {
            req = req.content_type(ct);
        }
        req = req.content_encoding("identity");

        let resp = traced_s3_request!(req)
            .send()
            .await
            .map_err(|e| StorageError::S3(e.to_string()))?;

        resp.upload_id()
            .map(str::to_owned)
            .ok_or_else(|| StorageError::S3("S3 did not return a multipart upload id".to_owned()))
    }

    async fn upload_part(
        &self,
        key: &str,
        multipart_upload_id: &str,
        part_number: i32,
        body: Vec<u8>,
    ) -> Result<UploadedPart, StorageError> {
        let size_bytes = i64::try_from(body.len())
            .map_err(|e| StorageError::S3(format!("invalid upload part size: {e}")))?;

        let resp = traced_s3_request!(
            self.client
                .upload_part()
                .bucket(&self.bucket)
                .key(key)
                .upload_id(multipart_upload_id)
                .part_number(part_number)
                .body(ByteStream::from(body))
        )
        .send()
        .await
        .map_err(|e| StorageError::S3(e.to_string()))?;

        let etag = resp
            .e_tag()
            .map(str::to_owned)
            .ok_or_else(|| StorageError::S3("S3 did not return an upload part ETag".to_owned()))?;

        Ok(UploadedPart {
            part_number,
            etag,
            size_bytes,
        })
    }

    async fn complete_multipart_upload(
        &self,
        key: &str,
        multipart_upload_id: &str,
        parts: &[CompletedUploadPart],
    ) -> Result<(), StorageError> {
        let completed_parts = parts
            .iter()
            .map(|part| {
                CompletedPart::builder()
                    .part_number(part.part_number)
                    .e_tag(&part.etag)
                    .build()
            })
            .collect::<Vec<_>>();

        let upload = CompletedMultipartUpload::builder()
            .set_parts(Some(completed_parts))
            .build();

        traced_s3_request!(
            self.client
                .complete_multipart_upload()
                .bucket(&self.bucket)
                .key(key)
                .upload_id(multipart_upload_id)
                .multipart_upload(upload)
        )
        .send()
        .await
        .map_err(|e| StorageError::S3(e.to_string()))?;

        Ok(())
    }

    async fn abort_multipart_upload(
        &self,
        key: &str,
        multipart_upload_id: &str,
    ) -> Result<(), StorageError> {
        traced_s3_request!(
            self.client
                .abort_multipart_upload()
                .bucket(&self.bucket)
                .key(key)
                .upload_id(multipart_upload_id)
        )
        .send()
        .await
        .map_err(|e| StorageError::S3(e.to_string()))?;

        Ok(())
    }

    async fn put_object(
        &self,
        key: &str,
        content_type: Option<&str>,
        body: Vec<u8>,
    ) -> Result<(), StorageError> {
        let mut req = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from(body));

        if let Some(ct) = content_type {
            req = req.content_type(ct);
        }

        traced_s3_request!(req).send().await?;

        Ok(())
    }

    async fn delete_objects(&self, keys: &[String]) -> Result<(), StorageError> {
        if keys.is_empty() {
            return Ok(());
        }

        let mut builder = Delete::builder();
        for key in keys {
            builder =
                builder.objects(ObjectIdentifier::builder().key(key).build().map_err(|e| {
                    StorageError::S3(format!("failed to build object identifier: {e}"))
                })?);
        }

        let delete = builder
            .build()
            .map_err(|e| StorageError::S3(format!("failed to build delete request: {e}")))?;

        let response = traced_s3_request!(
            self.client
                .delete_objects()
                .bucket(&self.bucket)
                .delete(delete)
        )
        .send()
        .await?;

        let errors = response.errors();
        if !errors.is_empty() {
            let first_error = errors
                .first()
                .and_then(|error| error.code())
                .unwrap_or("S3");
            return Err(StorageError::S3(format!(
                "failed to delete {} S3 object(s); first error code: {first_error}",
                errors.len()
            )));
        }

        debug!(count = keys.len(), bucket = %self.bucket, "S3 objects deleted");

        Ok(())
    }

    async fn head_object(&self, key: &str) -> Result<ObjectMeta, StorageError> {
        let resp = traced_s3_request!(self.client.head_object().bucket(&self.bucket).key(key))
            .send()
            .await?;

        Ok(ObjectMeta {
            size_bytes: resp.content_length().unwrap_or(0),
            etag: resp.e_tag().map(|s| s.to_string()),
            content_encoding: resp.content_encoding().map(str::to_owned),
        })
    }
}
