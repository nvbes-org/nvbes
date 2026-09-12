#[path = "account.error.rs"]
pub mod error;
#[path = "account.privacy.mod.rs"]
pub mod privacy;
#[path = "account.profile.mod.rs"]
pub mod profile;

pub use error::{AccountError, AccountResult};
#[path = "account.closure.event.rs"]
pub mod closure_event;
#[path = "account.export.event.rs"]
pub mod export_event;
