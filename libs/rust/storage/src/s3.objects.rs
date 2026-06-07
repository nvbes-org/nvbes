use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{Delete, ObjectIdentifier};

use crate::error::StorageError;
use crate::trait_def::{ByteRange, ObjectMeta};

use super::{S3ObjectStore, traced_s3_request};

pub(super) async fn get_object(store: &S3ObjectStore, key: &str) -> Result<Vec<u8>, StorageError> {
    let resp = traced_s3_request!(store.client.get_object().bucket(&store.bucket).key(key))
        .send()
        .await?;

    let data = resp
        .body
        .collect()
        .await
        .map_err(|e| StorageError::S3(format!("failed to read object body: {e}")))?;
    let bytes = data.into_bytes().to_vec();

    tracing::debug!(key, bucket = %store.bucket, size = bytes.len(), "S3 object downloaded");

    Ok(bytes)
}

pub(super) async fn get_object_range(
    store: &S3ObjectStore,
    key: &str,
    range: ByteRange,
) -> Result<Vec<u8>, StorageError> {
    let resp = traced_s3_request!(
        store
            .client
            .get_object()
            .bucket(&store.bucket)
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

    tracing::debug!(
        key,
        bucket = %store.bucket,
        start = range.start,
        end = range.end_inclusive,
        size = bytes.len(),
        "S3 object range downloaded"
    );

    Ok(bytes)
}

pub(super) async fn put_object(
    store: &S3ObjectStore,
    key: &str,
    content_type: Option<&str>,
    body: Vec<u8>,
) -> Result<(), StorageError> {
    let mut req = store
        .client
        .put_object()
        .bucket(&store.bucket)
        .key(key)
        .body(ByteStream::from(body));

    if let Some(ct) = content_type {
        req = req.content_type(ct);
    }

    traced_s3_request!(req).send().await?;
    Ok(())
}

pub(super) async fn delete_objects(
    store: &S3ObjectStore,
    keys: &[String],
) -> Result<(), StorageError> {
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
        store
            .client
            .delete_objects()
            .bucket(&store.bucket)
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

    tracing::debug!(count = keys.len(), bucket = %store.bucket, "S3 objects deleted");
    Ok(())
}

pub(super) async fn head_object(
    store: &S3ObjectStore,
    key: &str,
) -> Result<ObjectMeta, StorageError> {
    let resp = traced_s3_request!(store.client.head_object().bucket(&store.bucket).key(key))
        .send()
        .await?;

    Ok(ObjectMeta {
        size_bytes: resp.content_length().unwrap_or(0),
        etag: resp.e_tag().map(|s| s.to_string()),
        content_encoding: resp.content_encoding().map(str::to_owned),
    })
}
