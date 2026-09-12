use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};

use crate::error::StorageError;
use crate::trait_def::{CompletedUploadPart, UploadedPart};

use super::{S3ObjectStore, traced_s3_request};

pub(super) async fn create_multipart_upload(
    store: &S3ObjectStore,
    key: &str,
    content_type: Option<&str>,
) -> Result<String, StorageError> {
    let mut req = store
        .client
        .create_multipart_upload()
        .bucket(&store.bucket)
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

pub(super) async fn upload_part(
    store: &S3ObjectStore,
    key: &str,
    multipart_upload_id: &str,
    part_number: i32,
    body: Vec<u8>,
) -> Result<UploadedPart, StorageError> {
    let size_bytes = i64::try_from(body.len())
        .map_err(|e| StorageError::S3(format!("invalid upload part size: {e}")))?;

    let resp = traced_s3_request!(
        store
            .client
            .upload_part()
            .bucket(&store.bucket)
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

pub(super) async fn complete_multipart_upload(
    store: &S3ObjectStore,
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
        store
            .client
            .complete_multipart_upload()
            .bucket(&store.bucket)
            .key(key)
            .upload_id(multipart_upload_id)
            .multipart_upload(upload)
    )
    .send()
    .await
    .map_err(|e| StorageError::S3(e.to_string()))?;

    Ok(())
}

pub(super) async fn abort_multipart_upload(
    store: &S3ObjectStore,
    key: &str,
    multipart_upload_id: &str,
) -> Result<(), StorageError> {
    traced_s3_request!(
        store
            .client
            .abort_multipart_upload()
            .bucket(&store.bucket)
            .key(key)
            .upload_id(multipart_upload_id)
    )
    .send()
    .await
    .map_err(|e| StorageError::S3(e.to_string()))?;

    Ok(())
}
