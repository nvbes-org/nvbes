use std::fmt;

use nvbes_email::{EmailError, EmailFailureClass};
use nvbes_product_identity::{IdentityError, error::IdentityErrorKind};

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

    pub(super) fn from_email(error: &EmailError) -> Self {
        let class = match error.failure_class() {
            EmailFailureClass::Transient => JobFailureClass::Transient,
            EmailFailureClass::Permanent => JobFailureClass::Permanent,
        };
        Self::new(class, error.safe_code(), error.safe_summary())
    }

    pub(super) fn from_identity(error: &IdentityError) -> Self {
        let class = match error.kind {
            IdentityErrorKind::Internal => JobFailureClass::Transient,
            IdentityErrorKind::BadRequest
            | IdentityErrorKind::Unauthorized
            | IdentityErrorKind::Forbidden
            | IdentityErrorKind::NotFound
            | IdentityErrorKind::Conflict => JobFailureClass::Permanent,
        };
        let summary = match class {
            JobFailureClass::Transient => "Identity persistence operation failed",
            JobFailureClass::Permanent => "Identity job input or state is invalid",
        };
        Self::new(class, &error.code, summary)
    }

    pub(super) fn from_stored(class: &str, code: &str, summary: &str) -> Self {
        let class = if class == JobFailureClass::Permanent.as_str() {
            JobFailureClass::Permanent
        } else {
            JobFailureClass::Transient
        };
        Self::new(class, code, summary)
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

    pub(super) fn summary(&self) -> &str {
        &self.summary
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
    use nvbes_email::EmailError;
    use nvbes_product_identity::{IdentityError, IdentityErrorKind};

    #[test]
    fn provider_details_are_not_exposed_to_queue_errors() {
        let secret = "Authorization: Bearer secret-value";
        let provider_error = EmailError::Api {
            status: 503,
            message: secret.to_string(),
        };
        let failure = JobExecutionError::from_email(&provider_error);

        assert!(failure.is_retryable());
        assert!(!failure.to_string().contains(secret));
    }

    #[test]
    fn persisted_fields_are_single_line_and_bounded() {
        let failure = JobExecutionError::permanent(
            &format!("bad\n{}", "x".repeat(MAX_CODE_CHARS * 2)),
            &format!("unsafe\r\n{}", "y".repeat(MAX_SUMMARY_CHARS * 2)),
        );

        assert!(!failure.code().contains('\n'));
        assert!(!failure.summary().contains('\n'));
        assert!(failure.code().chars().count() <= MAX_CODE_CHARS);
        assert!(failure.summary().chars().count() <= MAX_SUMMARY_CHARS);
    }

    #[test]
    fn only_internal_identity_failures_are_retryable() {
        let transient = JobExecutionError::from_identity(&IdentityError::internal(
            "database_error",
            "temporary database failure",
        ));
        assert!(transient.is_retryable());

        for kind in [
            IdentityErrorKind::BadRequest,
            IdentityErrorKind::Unauthorized,
            IdentityErrorKind::Forbidden,
            IdentityErrorKind::NotFound,
            IdentityErrorKind::Conflict,
        ] {
            let permanent = JobExecutionError::from_identity(&IdentityError::new(
                kind,
                "invalid_job",
                "invalid identity job input",
            ));
            assert!(!permanent.is_retryable(), "{kind:?} must not be retried");
        }
    }
}
