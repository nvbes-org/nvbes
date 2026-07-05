use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct TenantView {
    pub id: Uuid,
    pub kind: String,
    pub name: String,
    pub slug: String,
    pub status: String,
    pub security_tier: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct OrganizationView {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub parent_organization_id: Option<Uuid>,
    pub name: String,
    pub slug: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct IdentityMembershipView {
    pub scope_type: String,
    pub scope_id: Uuid,
    pub principal_id: Uuid,
    pub role: String,
    pub status: String,
    pub source: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct IdentityInvitationView {
    pub id: Uuid,
    pub scope_type: String,
    pub scope_id: Uuid,
    pub email: String,
    pub role: String,
    pub status: String,
    pub expires_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct TenantResponse {
    pub tenant: TenantView,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct OrganizationsResponse {
    pub organizations: Vec<OrganizationView>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct OrganizationResponse {
    pub organization: OrganizationView,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct IdentityMembershipResponse {
    pub membership: IdentityMembershipView,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct IdentityInvitationResponse {
    pub invitation: IdentityInvitationView,
    pub invitation_token: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct UpdateTenantInput {
    pub name: Option<String>,
    pub security_tier: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct CreateOrganizationInput {
    pub name: String,
    pub slug: String,
    pub parent_organization_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct UpdateOrganizationInput {
    pub name: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct UpsertIdentityMembershipInput {
    pub role: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct InviteIdentityMemberInput {
    pub email: String,
    pub role: String,
    pub expires_in_days: Option<i64>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct AcceptIdentityInvitationInput {
    pub token: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct AcceptIdentityInvitationResponse {
    pub scope_type: String,
    pub scope_id: Uuid,
    pub role: String,
    pub accepted_at: DateTime<Utc>,
}
