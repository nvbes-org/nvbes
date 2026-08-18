pub mod client;
pub mod command;
pub mod error;
pub mod mock;
pub mod operations;
pub mod proto;
pub mod renderer;
pub mod smtp;
pub mod test_capture;
#[path = "trait.rs"]
pub mod trait_def;

#[cfg(test)]
#[path = "environment.test_support.rs"]
mod environment_test_support;

pub use client::{EmailClient, EmailClientConfig, EmailClientError};
pub use command::{
    AccountSecurityEvent, EmailCategory, EmailCommand, EmailCommandError, EmailIdempotencyKey,
    EmailReceipt, EmailRecipient, EmailRequestContext, EmailTemplate, normalized_timezone,
};
pub use error::{EmailError, EmailFailureClass};
pub use mock::MockEmailSender;
pub use operations::{EmailOperationsClient, EmailOperationsError, privacy_activity_json};
pub use renderer::RenderedEmail;
pub use smtp::{SmtpEmailConfig, SmtpEmailSender};
pub use test_capture::TestCaptureEmailSender;
pub use trait_def::{EmailAddress, EmailMessage, EmailSender, SendResult};
