#[path = "drive.app.rs"]
pub mod app;
pub use nvbes_product_cloud::db;
#[path = "drive.domains.mod.rs"]
pub mod domains;
#[path = "drive.grpc.mod.rs"]
pub mod grpc;
#[cfg(test)]
#[path = "drive.grpc.workspace.boundary.tests.rs"]
mod grpc_workspace_boundary_tests;
#[cfg(test)]
#[path = "drive.grpc.workspace.tests.rs"]
mod grpc_workspace_tests;
#[path = "drive.http.mod.rs"]
pub mod http;
#[cfg(test)]
#[path = "drive.test_support.db.rs"]
pub(crate) mod test_support;
