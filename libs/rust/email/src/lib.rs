pub mod error;
pub mod mock;
pub mod smtp;
#[path = "trait.rs"]
pub mod trait_def;

pub use error::EmailError;
pub use mock::MockEmailSender;
pub use smtp::{SmtpEmailConfig, SmtpEmailSender};
pub use trait_def::{EmailAddress, EmailMessage, EmailSender, SendResult};
