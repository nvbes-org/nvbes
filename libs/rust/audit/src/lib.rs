use serde_json::Value;
use uuid::Uuid;

/// Domain input for an append-only audit event.
///
/// Persistence belongs to the owning service schema (for example
/// `billing_audit_events`). Cloud-era `audit_events` inserts were removed
/// because that table is absent from active V1 migrations.
#[derive(Debug, Clone)]
pub struct AuditEventInput<'a> {
    pub tenant_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub actor_principal_id: Option<Uuid>,
    pub action: &'a str,
    pub target_type: &'a str,
    pub target_id: Option<Uuid>,
    pub ip: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub metadata: Value,
}

impl<'a> AuditEventInput<'a> {
    pub fn action_key(&self) -> &'a str {
        self.action
    }

    pub fn target_key(&self) -> &'a str {
        self.target_type
    }

    pub fn is_workspace_scoped(&self) -> bool {
        self.workspace_id.is_some()
    }

    pub fn has_actor(&self) -> bool {
        self.actor_principal_id.is_some()
    }

    pub fn has_network_context(&self) -> bool {
        self.ip.is_some() || self.user_agent.is_some()
    }
}

#[cfg(test)]
#[path = "audit.tests.rs"]
mod tests;
