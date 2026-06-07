use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct ParResponse {
    pub request_uri: String,
    pub expires_in: i64,
}
