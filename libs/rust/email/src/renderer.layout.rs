use chrono::{Datelike, Utc};

pub(super) enum TemplateHtml {
    Body(String),
    Document(String),
}

pub(super) fn render(preview: &str, subject: &str, html: TemplateHtml) -> String {
    match html {
        TemplateHtml::Body(content) => interpolate(
            EMAIL_SHELL,
            &[
                ("__NVBES_PREVIEW__", escape(preview)),
                ("__NVBES_TITLE__", escape(subject)),
                ("__NVBES_CONTENT__", content),
                ("__NVBES_YEAR__", Utc::now().year().to_string()),
            ],
        ),
        TemplateHtml::Document(document) => document,
    }
}

pub(super) struct ActionEmail<'a> {
    pub action_label: &'a str,
    pub action_url: &'a str,
    pub expiry: &'a str,
    pub greeting: &'a str,
    pub message: &'a str,
    pub subject: &'a str,
    pub timing: &'a str,
    pub timezone: &'a str,
}

pub(super) fn action_email(email: ActionEmail<'_>) -> String {
    interpolate(
        EMAIL_ACTION,
        &[
            ("__NVBES_ACTION_LABEL__", escape(email.action_label)),
            ("__NVBES_ACTION_URL__", escape(email.action_url)),
            ("__NVBES_EXPIRY__", escape(email.expiry)),
            ("__NVBES_GREETING__", escape(email.greeting)),
            ("__NVBES_MESSAGE__", escape(email.message)),
            ("__NVBES_PREVIEW__", escape(email.subject)),
            ("__NVBES_TIMING__", escape(email.timing)),
            ("__NVBES_TIMEZONE__", escape(email.timezone)),
            ("__NVBES_TITLE__", escape(email.subject)),
            ("__NVBES_YEAR__", Utc::now().year().to_string()),
        ],
    )
}

const EMAIL_ACTION: &str = include_str!("../templates/email.action.generated.html");
const EMAIL_SHELL: &str = include_str!("../templates/email.shell.generated.html");

fn interpolate(template: &str, values: &[(&str, String)]) -> String {
    let mut output = String::with_capacity(template.len());
    let mut remaining = template;

    while let Some((index, marker, value)) = values
        .iter()
        .filter_map(|(marker, value)| remaining.find(marker).map(|index| (index, marker, value)))
        .min_by_key(|(index, _, _)| *index)
    {
        output.push_str(&remaining[..index]);
        output.push_str(value);
        remaining = &remaining[index + marker.len()..];
    }
    output.push_str(remaining);
    output
}

pub(super) fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}
