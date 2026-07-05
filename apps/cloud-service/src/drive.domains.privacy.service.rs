#[path = "drive.domains.privacy.service.account.rs"]
mod account;
#[path = "drive.domains.privacy.service.download.rs"]
mod download;
#[path = "drive.domains.privacy.service.payload.rs"]
mod payload;
#[path = "drive.domains.privacy.service.policy.rs"]
mod policy;
#[path = "drive.domains.privacy.service.requests.rs"]
mod requests;
#[cfg(test)]
#[path = "drive.domains.privacy.service.tests.rs"]
mod tests;
#[path = "drive.domains.privacy.service.workspace.rs"]
mod workspace;

pub use super::types::{PrivacyRequestDraft, PrivacyRequestResponse, PrivacyRequestStatusResponse};
pub use account::{get_request, request_account_delete, request_account_export};
pub use workspace::{request_workspace_delete, request_workspace_export};

pub(super) const JOB_PRIVACY_ACCOUNT_EXPORT: &str = "privacy.account_export";
pub(super) const JOB_PRIVACY_ACCOUNT_DELETE: &str = "privacy.account_delete";
pub(super) const JOB_PRIVACY_WORKSPACE_EXPORT: &str = "privacy.workspace_export";
pub(super) const JOB_PRIVACY_WORKSPACE_DELETE: &str = "privacy.workspace_delete";
