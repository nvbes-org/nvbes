use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use super::types::MemberView;
use crate::domains::auth::types::derive_display_name;

#[derive(Debug, FromRow)]
pub struct MemberRecord {
    pub user_id: Uuid,
    pub email: String,
    pub firstname: Option<String>,
    pub lastname: Option<String>,
    pub username: Option<String>,
    pub role: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl MemberRecord {
    pub fn into_view(self) -> MemberView {
        let display_name = derive_display_name(
            self.firstname.as_deref(),
            self.lastname.as_deref(),
            self.username.as_deref(),
        );

        MemberView {
            user_id: self.user_id,
            email: self.email,
            display_name,
            role: self.role,
            status: self.status,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
