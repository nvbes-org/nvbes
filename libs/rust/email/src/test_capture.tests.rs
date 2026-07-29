use std::time::{SystemTime, UNIX_EPOCH};

use crate::{EmailAddress, EmailMessage, EmailSender};

use super::TestCaptureEmailSender;

#[tokio::test]
async fn captures_once_with_private_permissions_and_stable_provider_id() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "nvbes-email-capture-{}-{unique}",
        std::process::id()
    ));
    let sender = TestCaptureEmailSender::new(&directory).expect("capture sender");
    let job_id = "00000000-0000-0000-0000-000000000001";
    let message = EmailMessage {
        from: EmailAddress {
            email: "sender@example.test".to_string(),
            name: None,
        },
        to: vec![EmailAddress {
            email: "recipient@example.test".to_string(),
            name: None,
        }],
        subject: "Verification".to_string(),
        text_body: Some("token=secret".to_string()),
        html_body: None,
        headers: vec![
            ("X-Nvbes-Email-Job-Id".to_string(), job_id.to_string()),
            (
                "X-Nvbes-Email-Business-Type".to_string(),
                "verification".to_string(),
            ),
        ],
    };

    let first = sender.send_message(&message).await.expect("first capture");
    let second = sender
        .send_message(&message)
        .await
        .expect("deduplicated capture");
    assert_eq!(first.provider_email_id, second.provider_email_id);
    assert_eq!(
        std::fs::read_dir(&directory)
            .expect("capture directory")
            .count(),
        1
    );

    let path = directory.join(format!("capture-{job_id}.json"));
    let capture = std::fs::read_to_string(&path).expect("capture body");
    assert!(capture.contains("token=secret"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&path)
                .expect("capture metadata")
                .permissions()
                .mode()
                & 0o077,
            0
        );
    }
    std::fs::remove_dir_all(directory).expect("capture cleanup");
}

#[tokio::test]
async fn rejects_unsafe_job_identifiers() {
    let directory = std::env::temp_dir().join(format!(
        "nvbes-email-capture-invalid-{}",
        std::process::id()
    ));
    let sender = TestCaptureEmailSender::new(&directory).expect("capture sender");
    let result = sender
        .send_message(&EmailMessage {
            from: EmailAddress {
                email: "sender@example.test".to_string(),
                name: None,
            },
            to: Vec::new(),
            subject: "Subject".to_string(),
            text_body: None,
            html_body: None,
            headers: vec![
                (
                    "X-Nvbes-Email-Job-Id".to_string(),
                    "../../escape".to_string(),
                ),
                (
                    "X-Nvbes-Email-Business-Type".to_string(),
                    "verification".to_string(),
                ),
            ],
        })
        .await;
    assert!(result.is_err());
    std::fs::remove_dir_all(directory).expect("capture cleanup");
}
