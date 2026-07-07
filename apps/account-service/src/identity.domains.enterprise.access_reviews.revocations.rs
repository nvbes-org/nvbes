use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RevokedOAuthClient {
    pub id: Uuid,
    pub client_id: String,
}
