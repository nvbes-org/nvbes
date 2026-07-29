use crate::http::error::AppError;
use nvbes_email::{EmailAddress, EmailMessage};

pub(crate) fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn render_template(template: &str, replacements: &[(&str, &str)]) -> String {
    let mut result = template.to_string();
    for (key, value) in replacements {
        result = result.replace(key, &html_escape(value));
    }
    result
}

fn from_address(config: &crate::app::AppConfig) -> Result<EmailAddress, AppError> {
    let email = require_from_email(config)?;
    Ok(EmailAddress {
        email,
        name: Some(
            config
                .email_from_name
                .clone()
                .unwrap_or_else(|| "nvbes".to_string()),
        ),
    })
}

pub(crate) fn ensure_delivery_configured(config: &crate::app::AppConfig) -> Result<(), AppError> {
    require_from_email(config).map(|_| ())
}

fn require_from_email(config: &crate::app::AppConfig) -> Result<String, AppError> {
    if let Some(email) = config.email_from_email.clone() {
        return Ok(email);
    }

    if config.environment == "development" && config.email_provider == "mock" {
        return Ok("dev@nvbes.local".to_string());
    }

    Err(AppError::new(
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        "email_not_configured",
        "NVBES_EMAIL_FROM_EMAIL must be set to send emails.",
    ))
}

fn reply_to_header(config: &crate::app::AppConfig) -> Result<Vec<(String, String)>, AppError> {
    let reply_to = config
        .email_reply_to
        .clone()
        .map(Ok)
        .unwrap_or_else(|| require_from_email(config))?;
    Ok(vec![("Reply-To".to_string(), reply_to)])
}

const VERIFICATION_TEMPLATE: &str = include_str!("identity.email.templates.verification.html");
const PASSWORD_RESET_TEMPLATE: &str = include_str!("identity.email.templates.password-reset.html");
const PASSWORD_CHANGE_CODE_TEMPLATE: &str =
    include_str!("identity.email.templates.password-change-code.html");
const INVITATION_TEMPLATE: &str = include_str!("identity.email.templates.invitation.html");

pub fn verification_email(
    config: &crate::app::AppConfig,
    to_email: &str,
    to_name: &str,
    token: &str,
) -> Result<EmailMessage, AppError> {
    let link = format!("{}/verify-result?token={}", config.web_base_url, token);

    let html_body = render_template(
        VERIFICATION_TEMPLATE,
        &[
            ("{{user_name}}", to_name),
            ("{{verification_link}}", &link),
            (
                "{{expires_hours}}",
                &config.auth_verification_ttl_hours.to_string(),
            ),
        ],
    );

    Ok(EmailMessage {
        from: from_address(config)?,
        to: vec![EmailAddress {
            email: to_email.to_string(),
            name: Some(to_name.to_string()),
        }],
        subject: "Verify your email address".to_string(),
        text_body: Some(format!(
            "Welcome to nvbes!\n\nPlease verify your email by clicking this link:\n{}\n\nThis link expires in {} hours.",
            link, config.auth_verification_ttl_hours
        )),
        html_body: Some(html_body),
        headers: reply_to_header(config)?,
    })
}

pub fn password_reset_email(
    config: &crate::app::AppConfig,
    to_email: &str,
    to_name: &str,
    token: &str,
) -> Result<EmailMessage, AppError> {
    let link = format!("{}/reset-password?token={}", config.web_base_url, token);

    let html_body = render_template(
        PASSWORD_RESET_TEMPLATE,
        &[
            ("{{user_name}}", to_name),
            ("{{reset_link}}", &link),
            (
                "{{expires_minutes}}",
                &config.auth_password_reset_ttl_minutes.to_string(),
            ),
        ],
    );

    Ok(EmailMessage {
        from: from_address(config)?,
        to: vec![EmailAddress {
            email: to_email.to_string(),
            name: Some(to_name.to_string()),
        }],
        subject: "Reset your password".to_string(),
        text_body: Some(format!(
            "Reset your nvbes password by clicking this link:\n{}\n\nThis link expires in {} minutes.",
            link, config.auth_password_reset_ttl_minutes
        )),
        html_body: Some(html_body),
        headers: reply_to_header(config)?,
    })
}

pub fn password_change_code_email(
    config: &crate::app::AppConfig,
    to_email: &str,
    to_name: &str,
    code: &str,
    expires_minutes: i64,
) -> Result<EmailMessage, AppError> {
    let html_body = render_template(
        PASSWORD_CHANGE_CODE_TEMPLATE,
        &[
            ("{{user_name}}", to_name),
            ("{{verification_code}}", code),
            ("{{expires_minutes}}", &expires_minutes.to_string()),
        ],
    );
    Ok(EmailMessage {
        from: from_address(config)?,
        to: vec![EmailAddress {
            email: to_email.to_string(),
            name: Some(to_name.to_string()),
        }],
        subject: "Code de vérification pour votre mot de passe".to_string(),
        text_body: Some(format!(
            "Votre code de vérification nvbes est : {code}\n\nIl expire dans {expires_minutes} minutes. Si vous n'êtes pas à l'origine de cette demande, ignorez cet email."
        )),
        html_body: Some(html_body),
        headers: reply_to_header(config)?,
    })
}

pub fn invitation_email(
    config: &crate::app::AppConfig,
    to_email: &str,
    workspace_name: &str,
    inviter_name: &str,
    token: &str,
) -> Result<EmailMessage, AppError> {
    let link = format!("{}/join?token={}", config.web_base_url, token);

    let html_body = render_template(
        INVITATION_TEMPLATE,
        &[
            ("{{inviter_name}}", inviter_name),
            ("{{workspace_name}}", workspace_name),
            ("{{invitation_link}}", &link),
        ],
    );

    Ok(EmailMessage {
        from: from_address(config)?,
        to: vec![EmailAddress {
            email: to_email.to_string(),
            name: None,
        }],
        subject: format!("Invitation to join {}", workspace_name),
        text_body: Some(format!(
            "{} has invited you to join {} on nvbes.\n\nAccept invitation:\n{}",
            html_escape(inviter_name),
            html_escape(workspace_name),
            link
        )),
        html_body: Some(html_body),
        headers: reply_to_header(config)?,
    })
}

#[cfg(test)]
#[path = "identity.email.tests.rs"]
mod tests;
