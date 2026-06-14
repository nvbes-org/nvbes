use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

use super::super::{policy, types::*};

#[derive(Debug, FromRow)]
pub struct ActorAccessRow {
    pub role: String,
}

#[derive(Debug, FromRow)]
pub struct EnterpriseUserRow {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub workspace_ids: Vec<Uuid>,
    pub status: String,
    pub mfa_enabled: bool,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl EnterpriseUserRow {
    pub fn into_view(self) -> EnterpriseUser {
        let role = policy::role_from_db(&self.role);
        EnterpriseUser {
            module_grants: policy::grants_for_role(&role),
            role,
            id: self.id,
            email: self.email,
            display_name: self.display_name,
            workspace_ids: self.workspace_ids,
            status: self.status,
            mfa_enabled: self.mfa_enabled,
            last_seen_at: self.last_seen_at,
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct EnterpriseInvitationRow {
    pub id: Uuid,
    pub email: String,
    pub role: String,
    pub workspace_ids: Vec<Uuid>,
    pub status: String,
    pub invited_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl EnterpriseInvitationRow {
    pub fn into_view(self) -> EnterpriseInvitation {
        let role = policy::role_from_db(&self.role);
        EnterpriseInvitation {
            module_grants: policy::grants_for_role(&role),
            role,
            id: self.id,
            email: self.email,
            workspace_ids: self.workspace_ids,
            status: self.status,
            invited_at: self.invited_at,
            expires_at: self.expires_at,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct WorkspaceSummaryRow {
    pub id: Uuid,
    pub name: String,
    pub workspace_type: String,
    pub data_region: Option<String>,
    pub member_count: i64,
    pub storage_used_bytes: i64,
    pub created_at: DateTime<Utc>,
}

impl WorkspaceSummaryRow {
    pub fn into_view(self) -> EnterpriseWorkspaceSummary {
        EnterpriseWorkspaceSummary {
            id: self.id,
            name: self.name,
            workspace_type: self.workspace_type,
            data_region: self.data_region,
            member_count: self.member_count,
            storage_used_bytes: self.storage_used_bytes,
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct AuditEventRow {
    pub id: Uuid,
    pub event_type: String,
    pub actor_id: Option<Uuid>,
    pub actor_email: Option<String>,
    pub target_type: Option<String>,
    pub target_id: Option<Uuid>,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

impl AuditEventRow {
    pub fn into_view(self) -> EnterpriseAuditEvent {
        EnterpriseAuditEvent {
            id: self.id,
            event_type: self.event_type,
            actor_id: self.actor_id,
            actor_email: self.actor_email,
            target_type: self.target_type,
            target_id: self.target_id,
            metadata: serde_json::from_value(self.metadata).ok(),
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct BillingSummaryRow {
    pub code: String,
    pub name: String,
    pub status: String,
    pub billing_email: Option<String>,
}

#[derive(Debug, FromRow)]
pub struct DeveloperCredentialRow {
    pub id: Uuid,
    pub name: String,
    pub owner_email: Option<String>,
    pub scopes: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl DeveloperCredentialRow {
    pub fn into_view(self) -> EnterpriseDeveloperCredentialSummary {
        EnterpriseDeveloperCredentialSummary {
            id: self.id,
            name: self.name,
            owner_email: self.owner_email,
            scopes: self.scopes,
            last_used_at: None,
            created_at: self.created_at,
            expires_at: None,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct PolicySummaryRow {
    pub id: Uuid,
    pub name: String,
    pub category: String,
    pub enabled: bool,
    pub configuration: Value,
    pub updated_at: DateTime<Utc>,
}

impl PolicySummaryRow {
    pub fn into_view(self) -> EnterprisePolicySummary {
        let configuration = match serde_json::from_value(self.configuration) {
            Ok(configuration) => configuration,
            Err(_) => Default::default(),
        };
        EnterprisePolicySummary {
            id: self.id,
            name: self.name,
            category: self.category,
            enabled: self.enabled,
            configuration,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct SecuritySummaryRow {
    pub mfa_factor_count: i64,
    pub passkey_count: i64,
    pub high_risk_event_count: i64,
}

#[derive(Debug, FromRow)]
pub struct InvoiceRow {
    pub id: Uuid,
    pub status: String,
    pub amount_due_cents: i64,
    pub issued_at: DateTime<Utc>,
}

impl InvoiceRow {
    pub fn into_view(self) -> EnterpriseInvoice {
        EnterpriseInvoice {
            id: self.id.to_string(),
            status: self.status,
            amount_due_cents: self.amount_due_cents,
            currency: "usd".to_string(),
            issued_at: self.issued_at,
            hosted_invoice_url: None,
        }
    }
}
