#[path = "identity.domains.federation.provisioning.inbound.rs"]
pub mod inbound;
#[path = "identity.domains.federation.provisioning.principal.rs"]
pub mod principal;

pub use inbound::{handle_inbound_federation, jit_provision};
pub use principal::{first_workspace_context, resolve_or_create_principal};
