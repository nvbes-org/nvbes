use chrono::{DateTime, Utc};

use super::layout::{TemplateHtml, escape};
use super::localization::utc_date_time;

pub(super) fn render_code(
    user_name: &str,
    code: &str,
    expires_at: DateTime<Utc>,
) -> (String, String, TemplateHtml) {
    let expiry = utc_date_time(expires_at);
    let subject = "Your nvbes password change code".to_string();
    let text = format!(
        "Hi {user_name},\n\nYour password change code is {code}. It expires on {}.\n\nIf you did not request this, secure your account.",
        expiry.date
    );
    let html = format!(
        "<p>Hi {},</p><p>Your password change code is:</p><p class=\"code\">{}</p><p class=\"muted\">Expires on {}.</p><p>If you did not request this, secure your account.</p>",
        escape(user_name),
        escape(code),
        escape(&expiry.date)
    );
    (subject, text, TemplateHtml::Body(html))
}
