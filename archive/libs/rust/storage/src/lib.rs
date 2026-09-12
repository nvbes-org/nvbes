pub mod envelope;
pub mod error;
pub mod mock;
pub mod s3;
#[path = "trait_def.rs"]
pub mod trait_def;

pub use envelope::{
    EnvelopeEncryptedData, EnvelopeError, decrypt_envelope, derive_tenant_kek, encrypt_envelope,
    rotate_envelope,
};
pub use error::StorageError;
pub use mock::MockObjectStore;
pub use s3::S3ObjectStore;
pub use trait_def::{
    ByteRange, CompletedUploadPart, ObjectMeta, ObjectStore, PresignedUrl, UploadedPart,
};
