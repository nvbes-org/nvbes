#[path = "identity.domains.auth.db.account.rs"]
mod account;
#[path = "identity.domains.auth.db.authentication_policy.rs"]
mod authentication_policy;
#[path = "identity.domains.auth.db.availability.rs"]
mod availability;
#[path = "identity.domains.auth.db.emails.rs"]
pub mod emails;
#[path = "identity.domains.auth.db.factor_view.rs"]
mod factor_view;
#[path = "identity.domains.auth.db.record.rs"]
mod record;
#[path = "identity.domains.auth.db.registration_enrollment.rs"]
pub mod registration_enrollment;

pub use account::create_user_account;
pub use authentication_policy::{enable_passwordless_login, passwordless_login_enabled};
pub use availability::registration_email_exists;
pub use factor_view::map_factor_view;
pub use record::{UserRecord, fetch_user_record, fetch_user_view};
