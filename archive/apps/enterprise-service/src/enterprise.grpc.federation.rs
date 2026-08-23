use chrono::{DateTime, Utc};
use uuid::Uuid;

#[path = "enterprise.grpc.federation.domains.rs"]
mod domains;
#[path = "enterprise.grpc.federation.governance.rs"]
mod governance;
#[path = "enterprise.grpc.federation.providers.rs"]
mod providers;
#[path = "enterprise.grpc.federation.scim.rs"]
mod scim;

pub use domains::{configure_tenant_domain, delete_tenant_domain, verify_tenant_domain};
pub use governance::federation_governance;
pub use providers::{configure_federation_provider, delete_federation_provider};
pub use scim::{configure_scim_connector, delete_scim_connector};

pub(crate) fn empty_to_option(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

pub(crate) fn optional_uuid_string(value: Option<Uuid>) -> String {
    value.map(|id| id.to_string()).unwrap_or_default()
}

pub(crate) fn optional_time_string(value: Option<DateTime<Utc>>) -> String {
    value.map(|time| time.to_rfc3339()).unwrap_or_default()
}

pub(crate) fn time_string(value: DateTime<Utc>) -> String {
    value.to_rfc3339()
}
