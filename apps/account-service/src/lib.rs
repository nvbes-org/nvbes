#[path = "identity.app.rs"]
pub mod app;
#[cfg(test)]
#[path = "identity.boundary.source_guard.tests.rs"]
mod boundary_source_guard_tests;
#[path = "identity.database.rs"]
pub mod database;
#[path = "identity.domains.mod.rs"]
pub mod domains;
#[path = "identity.email.mod.rs"]
pub mod email;
#[path = "identity.grpc.pb.rs"]
pub mod grpc_pb;
#[path = "identity.http.mod.rs"]
pub mod http;
#[path = "identity.test_support.rs"]
pub mod test_support;
