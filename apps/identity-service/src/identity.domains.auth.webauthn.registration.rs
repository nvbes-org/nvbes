#[path = "identity.domains.auth.webauthn.registration.finish.rs"]
mod finish;
#[path = "identity.domains.auth.webauthn.registration.options.rs"]
mod options;
#[path = "identity.domains.auth.webauthn.registration.start.rs"]
mod start;
#[cfg(test)]
#[path = "identity.domains.auth.webauthn.registration.tests.rs"]
mod tests;

pub use finish::finish_registration;
pub use start::start_registration;
