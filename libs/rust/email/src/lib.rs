pub mod error;
pub mod mock;
pub mod scaleway;
#[path = "trait.rs"]
pub mod trait_def;

pub use error::EmailError;
pub use mock::MockEmailSender;
pub use scaleway::ScalewayEmailClient;
pub use trait_def::{EmailAddress, EmailMessage, EmailSender, SendResult};
