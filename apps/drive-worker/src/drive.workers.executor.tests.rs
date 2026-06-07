use super::dispatch::is_known_job_type;

#[test]
fn known_privacy_job_types_are_accepted() {
    assert!(is_known_job_type(
        super::super::privacy::delete::JOB_PRIVACY_ACCOUNT_DELETE
    ));
    assert!(is_known_job_type(
        super::super::privacy::delete::JOB_PRIVACY_WORKSPACE_DELETE
    ));
    assert!(is_known_job_type(
        super::super::privacy::export::JOB_PRIVACY_ACCOUNT_EXPORT
    ));
    assert!(is_known_job_type(
        super::super::privacy::export::JOB_PRIVACY_WORKSPACE_EXPORT
    ));
}

#[test]
fn unknown_job_types_are_rejected() {
    assert!(!is_known_job_type("unknown.job"));
}
