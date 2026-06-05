#[path = "identity.domains.auth.webauthn.authentication.rs"]
pub mod authentication;
#[path = "identity.domains.auth.webauthn.errors.rs"]
pub mod errors;
#[path = "identity.domains.auth.webauthn.login.rs"]
pub mod login;
#[path = "identity.domains.auth.webauthn.registration.rs"]
pub mod registration;
#[path = "identity.domains.auth.webauthn.setup.rs"]
pub mod setup;
#[path = "identity.domains.auth.webauthn.storage.rs"]
pub mod storage;
#[path = "identity.domains.auth.webauthn.types.rs"]
pub mod types;

pub use authentication::{finish_authentication, start_authentication};
pub use login::{finish_login_authentication, start_login_authentication};
pub use registration::{finish_registration, start_registration};
pub use setup::build_webauthn;
