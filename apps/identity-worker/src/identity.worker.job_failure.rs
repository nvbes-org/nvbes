use std::fmt;

use nvbes_email::EmailClientError;

const MAX_CODE_CHARS: usize = 64;
const MAX_SUMMARY_CHARS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum JobFailureClass {
    Transient,
    Permanent,
}

impl JobFailureClass {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Transient => "transient",
            Self::Permanent => "permanent",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct JobExecutionError {
    class: JobFailureClass,
    code: String,
    summary: String,
}

impl JobExecutionError {
    pub(super) fn transient(code: &str, summary: &str) -> Self {
        Self::new(JobFailureClass::Transient, code, summary)
    }

    pub(super) fn permanent(code: &str, summary: &str) -> Self {
        Self::new(JobFailureClass::Permanent, code, summary)
    }

    pub(super) fn from_email_client(error: &EmailClientError) -> Self {
        match error {
            EmailClientError::Unavailable => Self::transient(
                "email_service_unavailable",
                "Global email command service is unavailable",
            ),
            EmailClientError::InvalidCommand(_) => {
                Self::permanent("email_command_invalid", "Email command is invalid")
            }
            EmailClientError::Conflict => Self::permanent(
                "email_command_conflict",
                "Email idempotency key conflicts with another command",
            ),
            EmailClientError::Unauthorized => Self::permanent(
                "email_service_unauthorized",
                "Email command producer is not authorized",
            ),
            EmailClientError::Protocol | EmailClientError::Configuration(_) => Self::permanent(
                "email_client_invalid",
                "Email command client is incorrectly configured",
            ),
        }
    }

    pub(super) const fn is_retryable(&self) -> bool {
        matches!(self.class, JobFailureClass::Transient)
    }

    pub(super) const fn class(&self) -> JobFailureClass {
        self.class
    }

    pub(super) fn code(&self) -> &str {
        &self.code
    }

    fn new(class: JobFailureClass, code: &str, summary: &str) -> Self {
        Self {
            class,
            code: bounded_single_line(code, MAX_CODE_CHARS),
            summary: bounded_single_line(summary, MAX_SUMMARY_CHARS),
        }
    }
}

impl fmt::Display for JobExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}:{}",
            self.class.as_str(),
            self.code,
            self.summary
        )
    }
}

impl std::error::Error for JobExecutionError {}

fn bounded_single_line(value: &str, maximum_chars: usize) -> String {
    let mut output = String::with_capacity(value.len().min(maximum_chars));
    let mut pending_space = false;
    let mut written = 0;

    for character in value.chars() {
        if character.is_whitespace() || character.is_control() {
            pending_space = !output.is_empty();
            continue;
        }
        if pending_space && written < maximum_chars {
            output.push(' ');
            written += 1;
            pending_space = false;
        }
        if written >= maximum_chars {
            break;
        }
        output.push(character);
        written += 1;
    }

    output
}

#[cfg(test)]
mod tests {
    use super::{JobExecutionError, MAX_CODE_CHARS, MAX_SUMMARY_CHARS};

    #[test]
    fn persisted_fields_are_single_line_and_bounded() {
        let failure = JobExecutionError::permanent(
            &format!("bad\n{}", "x".repeat(MAX_CODE_CHARS * 2)),
            &format!("unsafe\r\n{}", "y".repeat(MAX_SUMMARY_CHARS * 2)),
        );

        assert!(!failure.code().contains('\n'));
        assert!(!failure.summary.contains('\n'));
        assert!(failure.code().chars().count() <= MAX_CODE_CHARS);
        assert!(failure.summary.chars().count() <= MAX_SUMMARY_CHARS);
    }
}
