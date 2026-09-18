use aws_sdk_s3::presigning::PresigningConfig;

use crate::error::StorageError;
use crate::trait_def::PresignedUrl;

use super::S3ObjectStore;

pub(super) async fn presign_upload(
    store: &S3ObjectStore,
    key: &str,
    content_type: Option<&str>,
    _expected_size_bytes: i64,
    expires: std::time::Duration,
) -> Result<PresignedUrl, StorageError> {
    let presign_config = PresigningConfig::expires_in(expires)?;

    let mut req = store
        .presign_client
        .put_object()
        .bucket(&store.bucket)
        .key(key);

    if let Some(ct) = content_type {
        req = req.content_type(ct);
    }
    let presigned = req.presigned(presign_config).await?;

    tracing::debug!(key, bucket = %store.bucket, expires_secs = expires.as_secs(), "S3 presigned upload URL generated");

    Ok(PresignedUrl {
        url: presigned.uri().to_string(),
        method: "PUT",
        expires_in: expires,
    })
}

pub(super) async fn presign_download(
    store: &S3ObjectStore,
    key: &str,
    expires: std::time::Duration,
) -> Result<PresignedUrl, StorageError> {
    let presign_config = PresigningConfig::expires_in(expires)?;

    let presigned = store
        .presign_client
        .get_object()
        .bucket(&store.bucket)
        .key(key)
        .presigned(presign_config)
        .await?;

    tracing::debug!(key, bucket = %store.bucket, expires_secs = expires.as_secs(), "S3 presigned download URL generated");

    Ok(PresignedUrl {
        url: presigned.uri().to_string(),
        method: "GET",
        expires_in: expires,
    })
}
