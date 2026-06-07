#[path = "identity.domains.auth.db.account.rs"]
mod account;
#[path = "identity.domains.auth.db.factor_view.rs"]
mod factor_view;
#[path = "identity.domains.auth.db.preferences.rs"]
mod preferences;
#[path = "identity.domains.auth.db.profile.rs"]
mod profile;
#[path = "identity.domains.auth.db.record.rs"]
mod record;

pub use account::create_user_account;
pub use factor_view::map_factor_view;
pub use preferences::{
    fetch_user_notifications, fetch_user_preferences, update_user_notifications,
    update_user_preferences,
};
pub use profile::update_user_profile;
pub use record::{UserRecord, fetch_user_record, fetch_user_view};
