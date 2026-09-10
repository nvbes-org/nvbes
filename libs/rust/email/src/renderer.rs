use chrono::{DateTime, Utc};

use crate::{AccountSecurityEvent, EmailTemplate};

#[path = "renderer.code.rs"]
mod code;
#[path = "renderer.layout.rs"]
mod layout;
#[path = "renderer.localization.rs"]
mod localization;

use layout::{ActionEmail, TemplateHtml};
use localization::local_date_time;

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
                timezone,
            } => action(
                "Verify your email address",
                &format!(
                    "Hi {user_name},\n\nVerify your email address to activate your nvbes account."
                ),
                "Verify email",
                verification_url,
                "This link expires on",
                *credential_expires_at,
                timezone,
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
                "This link expires on",
                *credential_expires_at,
                "UTC",
            ),
            Self::PasswordChangeCodeV1 {
                user_name,
                code,
                credential_expires_at,
            } => code::render_code(user_name, code, *credential_expires_at),
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
                "This review is due on",
                *review_due_at,
                "UTC",
            ),
            Self::OperationalReadinessV1 {
                check_id,
                environment,
            } => operational_readiness(check_id, environment),
        };
        let preview_text = text.lines().next().unwrap_or(&subject).to_string();
        RenderedEmail {
            html_body: layout::render(&preview_text, &subject, content),
            subject,
            text_body: text,
            preview_text,
        }
    }
}

fn operational_readiness(check_id: &str, environment: &str) -> (String, String, TemplateHtml) {
    let subject = "nvbes email delivery test — no action required";
    let statement = format!(
        "This is an authorized nvbes email delivery test for {environment}. No action is required. Check ID: {check_id}."
    );
    (
        subject.to_string(),
        statement.clone(),
        TemplateHtml::Body(format!("<p>{}</p>", escape(&statement))),
    )
}

fn action(
    subject: &str,
    intro: &str,
    label: &str,
    url: &str,
    timing: &str,
    expires_at: DateTime<Utc>,
    timezone: &str,
) -> (String, String, TemplateHtml) {
    let expiry = local_date_time(expires_at, timezone);
    let text = format!(
        "{intro}\n\n{label}: {url}\n\n{timing} {}.\nTime shown in {}.",
        expiry.date, expiry.timezone
    );
    let mut paragraphs = intro.splitn(2, "\n\n");
    let greeting = paragraphs.next().unwrap_or_default();
    let message = paragraphs.next().unwrap_or_default();
    let html = layout::action_email(ActionEmail {
        action_label: label,
        action_url: url,
        expiry: &expiry.date,
        greeting,
        message,
        subject,
        timing,
        timezone: &expiry.timezone,
    });
    (subject.to_string(), text, TemplateHtml::Document(html))
}

fn security(
    event: AccountSecurityEvent,
    affected: &Option<String>,
    previous: &Option<String>,
    security_url: &Option<String>,
) -> (String, String, TemplateHtml) {
    let (subject, statement) = match event {
        AccountSecurityEvent::MfaRecoveryCodesGenerated => (
            "New recovery codes for your nvbes account",
            "A new set of MFA recovery codes was generated. Previous recovery codes can no longer be used. Keep the new codes offline; never share them.".to_string(),
        ),
        AccountSecurityEvent::MfaRecoveryStarted => (
            "MFA recovery started on your nvbes account",
            "A saved recovery code was used and existing sign-in sessions were revoked. Replacement of your authentication factors is not yet complete.".to_string(),
        ),
        AccountSecurityEvent::MfaRecovered => (
            "Authentication factors replaced on your nvbes account",
            "A new passkey replaced your previous authentication factors. Existing sessions and remaining recovery codes were revoked. Sign in again with the new passkey and save a new set of recovery codes. Your password was not changed by this recovery.".to_string(),
        ),
        AccountSecurityEvent::MfaRecoveryCancelled => (
            "MFA recovery cancelled on your nvbes account",
            "The MFA recovery was cancelled without replacing your authentication factors. The recovery code that was used remains consumed, and revoked sessions remain closed.".to_string(),
        ),
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
    (subject.to_string(), text, TemplateHtml::Body(html))
}

fn billing(
    amount: i64,
    currency: &str,
    invoice_url: Option<&str>,
    provider: Option<&str>,
) -> (String, String, TemplateHtml) {
    let subject = "Receipt for your nvbes subscription";
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
        TemplateHtml::Body(format!("<p>{}</p>{html_link}", escape(&statement))),
    )
}

fn payment_failure(
    amount: i64,
    currency: &str,
    portal: Option<&str>,
    invoice: Option<&str>,
    provider: Option<&str>,
) -> (String, String, TemplateHtml) {
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
        TemplateHtml::Body(format!(
            "<p>{}</p>{portal_html}{invoice_html}",
            escape(&statement)
        )),
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
