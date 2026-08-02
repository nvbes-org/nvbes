use chrono::{DateTime, Utc};

use crate::{AccountSecurityEvent, EmailTemplate};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedEmail {
    pub subject: String,
    pub text_body: String,
    pub html_body: String,
    pub preview_text: String,
}

impl EmailTemplate {
    pub fn render(&self) -> RenderedEmail {
        let (subject, text, content) = match self {
            Self::EmailVerificationV1 {
                user_name,
                verification_url,
                credential_expires_at,
            } => action(
                "Verify your email address",
                &format!(
                    "Hi {user_name},\n\nVerify your email address to activate your nvbes account."
                ),
                "Verify email",
                verification_url,
                Some(*credential_expires_at),
            ),
            Self::PasswordResetV1 {
                user_name,
                reset_url,
                credential_expires_at,
            } => action(
                "Reset your nvbes password",
                &format!("Hi {user_name},\n\nWe received a request to reset your nvbes password."),
                "Reset password",
                reset_url,
                Some(*credential_expires_at),
            ),
            Self::PasswordChangeCodeV1 {
                user_name,
                code,
                credential_expires_at,
            } => render_code(user_name, code, *credential_expires_at),
            Self::AccountSecurityV1 {
                event,
                affected_email,
                previous_email,
                security_url,
            } => security(*event, affected_email, previous_email, security_url),
            Self::BillingReceiptV1 {
                amount_minor,
                currency,
                invoice_url,
                provider_name,
                ..
            } => billing(
                true,
                *amount_minor,
                currency,
                invoice_url.as_deref(),
                provider_name.as_deref(),
            ),
            Self::BillingPaymentFailureV1 {
                amount_minor,
                currency,
                billing_portal_url,
                invoice_url,
                provider_name,
                ..
            } => payment_failure(
                *amount_minor,
                currency,
                billing_portal_url.as_deref(),
                invoice_url.as_deref(),
                provider_name.as_deref(),
            ),
            Self::AccessReviewReminderV1 {
                reviewer_name,
                campaign_name,
                review_url,
                review_due_at,
            } => action(
                "Access review reminder",
                &format!("Hi {reviewer_name},\n\nYour access review for {campaign_name} is due."),
                "Review access",
                review_url,
                Some(*review_due_at),
            ),
        };
        let preview_text = text.lines().next().unwrap_or(&subject).to_string();
        RenderedEmail {
            html_body: envelope(&preview_text, &subject, &content),
            subject,
            text_body: text,
            preview_text,
        }
    }
}

fn action(
    subject: &str,
    intro: &str,
    label: &str,
    url: &str,
    expires_at: Option<DateTime<Utc>>,
) -> (String, String, String) {
    let expiry = expires_at
        .map(|value| format!("\n\nThis link expires at {}.", value.to_rfc3339()))
        .unwrap_or_default();
    let text = format!("{intro}\n\n{label}: {url}{expiry}");
    let html_expiry = expires_at
        .map(|value| {
            format!(
                "<p class=\"muted\">Expires at {}.</p>",
                escape(&value.to_rfc3339())
            )
        })
        .unwrap_or_default();
    let content = format!(
        "<p>{}</p><p><a class=\"button\" href=\"{}\">{}</a></p>{html_expiry}",
        escape(&intro.replace("\n\n", " ")),
        escape(url),
        escape(label),
    );
    (subject.to_string(), text, content)
}

fn render_code(user_name: &str, code: &str, expires_at: DateTime<Utc>) -> (String, String, String) {
    let subject = "Your nvbes password change code".to_string();
    let text = format!(
        "Hi {user_name},\n\nYour password change code is {code}. It expires at {}.\n\nIf you did not request this, secure your account.",
        expires_at.to_rfc3339()
    );
    let html = format!(
        "<p>Hi {},</p><p>Your password change code is:</p><p class=\"code\">{}</p><p class=\"muted\">Expires at {}.</p><p>If you did not request this, secure your account.</p>",
        escape(user_name),
        escape(code),
        escape(&expires_at.to_rfc3339())
    );
    (subject, text, html)
}

fn security(
    event: AccountSecurityEvent,
    affected: &Option<String>,
    previous: &Option<String>,
    security_url: &Option<String>,
) -> (String, String, String) {
    let (subject, statement) = match event {
        AccountSecurityEvent::EmailAdded => (
            "New email added to your nvbes account",
            format!("A secondary email address was added: {}.", optional_value(affected)),
        ),
        AccountSecurityEvent::PrimaryEmailChanged => (
            "Primary email changed on your nvbes account",
            format!("Your primary email changed from {} to {}.", optional_value(previous), optional_value(affected)),
        ),
        AccountSecurityEvent::AccountRecovered => (
            "Your nvbes account was recovered",
            "Your password was reset and existing sessions and trusted devices were revoked.".to_string(),
        ),
        AccountSecurityEvent::RecoveryReviewRequired => (
            "Account recovery requires security review",
            "A high-risk recovery attempt is awaiting controlled security review. No reset token was issued.".to_string(),
        ),
    };
    let action = security_url
        .as_ref()
        .map(|url| format!("\n\nSecure your account: {url}"))
        .unwrap_or_default();
    let text =
        format!("{statement}\n\nIf this was not you, secure your account immediately.{action}");
    let link = security_url
        .as_ref()
        .map(|url| {
            format!(
                "<p><a class=\"button\" href=\"{}\">Secure account</a></p>",
                escape(url)
            )
        })
        .unwrap_or_default();
    let html = format!(
        "<p>{}</p><p>If this was not you, secure your account immediately.</p>{link}",
        escape(&statement)
    );
    (subject.to_string(), text, html)
}

fn billing(
    paid: bool,
    amount: i64,
    currency: &str,
    invoice_url: Option<&str>,
    provider: Option<&str>,
) -> (String, String, String) {
    let subject = if paid {
        "Receipt for your nvbes subscription"
    } else {
        "nvbes billing update"
    };
    let provider = provider
        .map(|value| format!(" through {value}"))
        .unwrap_or_default();
    let statement = format!(
        "We received your nvbes subscription payment of {}{provider}.",
        money(amount, currency)
    );
    let (text_link, html_link) = optional_link("Invoice", "View invoice", invoice_url);
    (
        subject.to_string(),
        format!("{statement}{text_link}"),
        format!("<p>{}</p>{html_link}", escape(&statement)),
    )
}

fn payment_failure(
    amount: i64,
    currency: &str,
    portal: Option<&str>,
    invoice: Option<&str>,
    provider: Option<&str>,
) -> (String, String, String) {
    let provider = provider
        .map(|value| format!(" through {value}"))
        .unwrap_or_default();
    let statement = format!(
        "We could not collect your nvbes subscription payment of {}{provider}.",
        money(amount, currency)
    );
    let (portal_text, portal_html) =
        optional_link("Billing portal", "Update payment method", portal);
    let (invoice_text, invoice_html) = optional_link("Invoice", "View invoice", invoice);
    (
        "Payment failed for your nvbes subscription".to_string(),
        format!("{statement}{portal_text}{invoice_text}"),
        format!("<p>{}</p>{portal_html}{invoice_html}", escape(&statement)),
    )
}

fn optional_link(text_label: &str, html_label: &str, url: Option<&str>) -> (String, String) {
    match url {
        Some(url) => (
            format!("\n\n{text_label}: {url}"),
            format!(
                "<p><a href=\"{}\">{}</a></p>",
                escape(url),
                escape(html_label)
            ),
        ),
        None => (String::new(), String::new()),
    }
}

fn money(amount_minor: i64, currency: &str) -> String {
    format!(
        "{}.{:02} {}",
        amount_minor / 100,
        amount_minor % 100,
        currency.to_ascii_uppercase()
    )
}

fn optional_value(value: &Option<String>) -> &str {
    value.as_deref().unwrap_or("the configured address")
}

fn envelope(preview: &str, subject: &str, content: &str) -> String {
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>{}</title></head><body style=\"background:#f5f7fb;font-family:Arial,sans-serif;color:#172033\"><span style=\"display:none\">{}</span><main style=\"max-width:600px;margin:24px auto;background:#fff;padding:32px\"><h1>{}</h1>{content}<hr><p class=\"muted\">nvbes</p></main></body></html>",
        escape(subject),
        escape(preview),
        escape(subject)
    )
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

#[cfg(test)]
#[path = "renderer.tests.rs"]
mod tests;
