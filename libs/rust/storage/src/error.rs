use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("S3 client error: {0}")]
    S3(String),

    #[error("presigning error: {0}")]
    Presign(String),

    #[error("object not found: {key}")]
    NotFound { key: String },

    #[error("configuration error: {0}")]
    Config(String),

    #[error("storage error: {0}")]
    Other(String),
}

impl From<aws_sdk_s3::error::SdkError<aws_sdk_s3::operation::get_object::GetObjectError>>
    for StorageError
{
    fn from(
        err: aws_sdk_s3::error::SdkError<aws_sdk_s3::operation::get_object::GetObjectError>,
    ) -> Self {
        if let aws_sdk_s3::error::SdkError::ServiceError(ref e) = err {
            if e.err().is_no_such_key() {
                return Self::NotFound { key: String::new() };
            }
        }
        Self::S3(err.to_string())
    }
}

impl From<aws_sdk_s3::error::SdkError<aws_sdk_s3::operation::head_object::HeadObjectError>>
    for StorageError
{
    fn from(
        err: aws_sdk_s3::error::SdkError<aws_sdk_s3::operation::head_object::HeadObjectError>,
    ) -> Self {
        if let aws_sdk_s3::error::SdkError::ServiceError(ref e) = err {
            if e.err().is_not_found() {
                return Self::NotFound { key: String::new() };
            }
        }
        Self::S3(err.to_string())
    }
}

impl From<aws_sdk_s3::error::SdkError<aws_sdk_s3::operation::delete_objects::DeleteObjectsError>>
    for StorageError
{
    fn from(
        err: aws_sdk_s3::error::SdkError<aws_sdk_s3::operation::delete_objects::DeleteObjectsError>,
    ) -> Self {
        Self::S3(err.to_string())
    }
}

impl From<aws_sdk_s3::presigning::PresigningConfigError> for StorageError {
    fn from(err: aws_sdk_s3::presigning::PresigningConfigError) -> Self {
        Self::Presign(err.to_string())
    }
}

impl From<aws_sdk_s3::error::SdkError<aws_sdk_s3::operation::put_object::PutObjectError>>
    for StorageError
{
    fn from(
        err: aws_sdk_s3::error::SdkError<aws_sdk_s3::operation::put_object::PutObjectError>,
    ) -> Self {
        Self::S3(err.to_string())
    }
}
