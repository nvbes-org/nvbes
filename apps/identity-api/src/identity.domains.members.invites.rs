use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use crate::domains::members::types::InvitationView;

pub use logic::{accept_invitation, invite_member};

#[path = "identity.domains.members.invites.logic.rs"]
pub mod logic;

#[derive(Debug, FromRow)]
pub struct InvitationRecord {
    pub id: Uuid,
    pub email: String,
    pub role: String,
    pub status: String,
    pub expires_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl InvitationRecord {
    pub fn into_view(self) -> InvitationView {
        InvitationView {
            id: self.id,
            email: self.email,
            role: self.role,
            status: self.status,
            expires_at: self.expires_at,
            accepted_at: self.accepted_at,
            revoked_at: self.revoked_at,
            created_at: self.created_at,
        }
    }
}
