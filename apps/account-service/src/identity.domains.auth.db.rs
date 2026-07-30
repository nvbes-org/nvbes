#[path = "identity.domains.auth.db.account.rs"]
mod account;
#[path = "identity.domains.auth.db.availability.rs"]
mod availability;
#[path = "identity.domains.auth.db.avatar.rs"]
mod avatar;
#[path = "identity.domains.auth.db.emails.rs"]
pub mod emails;
#[path = "identity.domains.auth.db.factor_view.rs"]
mod factor_view;
#[path = "identity.domains.auth.db.preferences.rs"]
mod preferences;
#[path = "identity.domains.auth.db.profile.rs"]
mod profile;
#[path = "identity.domains.auth.db.record.rs"]
mod record;
#[path = "identity.domains.auth.db.registration_enrollment.rs"]
pub mod registration_enrollment;

pub use account::create_user_account;
pub use availability::{registration_email_exists, registration_username_exists};
pub use avatar::{clear_profile_avatar, fetch_profile_avatar, save_profile_avatar};
pub use factor_view::map_factor_view;
pub use preferences::{
    fetch_user_notifications, fetch_user_preferences, update_user_notifications,
    update_user_preferences,
};
pub use profile::update_user_profile;
pub use record::{UserRecord, fetch_user_record, fetch_user_view};
