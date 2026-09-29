#[path = "security.cors.rs"]
mod cors;
#[path = "security.headers.rs"]
pub mod headers;
#[path = "security.profile.rs"]
pub mod profile;
#[path = "security.resource_cors.rs"]
pub mod resource_cors;
#[path = "security.tls_pinning.rs"]
pub mod tls_pinning;

pub use cors::cors_layer;
pub use headers::{
    insert_cdn_cache_headers, insert_clear_site_data_header, no_cache_headers, security_headers,
};
pub use profile::{DataClassification, SecurityProfile, TenantCellKind};
pub use tls_pinning::pinned_http_client;
