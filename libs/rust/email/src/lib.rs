pub mod error;
pub mod mock;
pub mod smtp;
pub mod test_capture;
#[path = "trait.rs"]
pub mod trait_def;

pub use error::{EmailError, EmailFailureClass};
pub use mock::MockEmailSender;
pub use smtp::{SmtpEmailConfig, SmtpEmailSender};
pub use test_capture::TestCaptureEmailSender;
pub use trait_def::{EmailAddress, EmailMessage, EmailSender, SendResult};
