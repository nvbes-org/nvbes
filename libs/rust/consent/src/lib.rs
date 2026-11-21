#[path = "consent.db.rs"]
pub mod db;
#[path = "consent.error.rs"]
pub mod error;
#[path = "consent.models.rs"]
pub mod models;
#[path = "consent.port.rs"]
pub mod port;
#[path = "consent.service.rs"]
pub mod service;

#[cfg(feature = "http")]
#[path = "consent.routes.rs"]
pub mod routes;

#[cfg(test)]
#[path = "consent.tests.rs"]
mod tests;

pub use error::ConsentError;
pub use models::*;
pub use port::ConsentPort;
pub use service::ConsentService;
