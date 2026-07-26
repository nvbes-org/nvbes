#[path = "identity.http.middleware.csrf.rs"]
pub mod csrf;
#[path = "identity.http.middleware.dpop.rs"]
pub mod dpop;
#[path = "identity.http.middleware.idempotency.rs"]
pub mod idempotency;
#[path = "identity.http.middleware.jwt.rs"]
pub mod jwt;
#[path = "identity.http.middleware.origin.rs"]
pub mod origin;
#[path = "identity.http.middleware.region.rs"]
pub mod region;

const PUBLIC_REPORT_PATHS: &[&str] = &["/csp-report", "/observability/network-errors"];
