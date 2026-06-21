use crate::error::PortError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectWrite {
    pub bucket: String,
    pub key: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectRead {
    pub bucket: String,
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectDelete {
    pub bucket: String,
    pub key: String,
}

pub trait ObjectStorePort {
    fn put(&mut self, write: ObjectWrite) -> Result<(), PortError>;
    fn get(&self, read: ObjectRead) -> Result<Vec<u8>, PortError>;
    fn delete(&mut self, delete: ObjectDelete) -> Result<(), PortError>;
}

impl ObjectWrite {
    pub fn validate(&self) -> Result<(), PortError> {
        if self.bucket.trim().is_empty() || self.key.trim().is_empty() {
            return Err(PortError::OperationFailed(
                "bucket and key are required".to_string(),
            ));
        }
        Ok(())
    }
}
