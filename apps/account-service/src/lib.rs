#[path = "identity.app.rs"]
pub mod app;
#[cfg(test)]
#[path = "identity.boundary.source_guard.tests.rs"]
mod boundary_source_guard_tests;
#[path = "identity.cloud_boundary.mod.rs"]
pub mod cloud_boundary;
#[path = "identity.database.rs"]
pub mod database;
#[cfg(test)]
#[path = "identity.database.migrations.tests.rs"]
mod database_migrations_tests;
#[cfg(test)]
#[path = "identity.database.rls.tests.rs"]
mod database_rls_tests;
#[path = "identity.developer.client.rs"]
pub mod developer_client;
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
