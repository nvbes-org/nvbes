use std::time::Duration;

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
    time::timeout,
};

use super::{SmtpEmailConfig, SmtpEmailSender};
use crate::{EmailAddress, EmailError, EmailFailureClass, EmailMessage, EmailSender, SendResult};

const TEST_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone, Copy)]
enum Scenario {
    Delivery,
    AuthenticatedDelivery,
    NoStartTls,
    RejectMail(&'static str),
}

#[derive(Debug)]
struct Transcript {
    commands: Vec<String>,
    data: String,
}

async fn spawn_server(scenario: Scenario) -> (u16, JoinHandle<Transcript>) {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind fake SMTP server");
    let port = listener.local_addr().expect("local SMTP address").port();
    let handle = tokio::spawn(async move {
        let (stream, _) = timeout(TEST_TIMEOUT, listener.accept())
            .await
            .expect("SMTP client should connect")
            .expect("accept SMTP client");
        timeout(TEST_TIMEOUT, serve_connection(stream, scenario))
            .await
            .expect("SMTP exchange should finish")
            .expect("serve SMTP exchange")
    });
    (port, handle)
}

async fn serve_connection(stream: TcpStream, scenario: Scenario) -> std::io::Result<Transcript> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut transcript = Transcript {
        commands: Vec::new(),
        data: String::new(),
    };

    writer
        .write_all(b"220 localhost ESMTP nvbes-test\r\n")
        .await?;
    let ehlo = read_line(&mut reader).await?;
    transcript.commands.push(ehlo.clone());
    if !ehlo.starts_with("EHLO ") {
        return Err(protocol_error("expected EHLO"));
    }

    match scenario {
        Scenario::AuthenticatedDelivery => {
            writer
                .write_all(b"250-localhost\r\n250 AUTH PLAIN\r\n")
                .await?;
            let auth = read_line(&mut reader).await?;
            transcript.commands.push(auth.clone());
            if auth != "AUTH PLAIN AHVzZXIAcGFzcw==" {
                writer
                    .write_all(b"535 5.7.8 invalid credentials\r\n")
                    .await?;
                return Ok(transcript);
            }
            writer.write_all(b"235 2.7.0 authenticated\r\n").await?;
        }
        Scenario::NoStartTls => {
            writer
                .write_all(b"250-localhost\r\n250 8BITMIME\r\n")
                .await?;
            if let Ok(Ok(command)) =
                timeout(Duration::from_millis(500), read_line(&mut reader)).await
                && !command.is_empty()
            {
                transcript.commands.push(command);
            }
            return Ok(transcript);
        }
        Scenario::Delivery | Scenario::RejectMail(_) => {
            writer
                .write_all(b"250-localhost\r\n250 8BITMIME\r\n")
                .await?;
        }
    }

    let mail_from = read_line(&mut reader).await?;
    transcript.commands.push(mail_from.clone());
    if !mail_from.starts_with("MAIL FROM:<sender@example.test>") {
        return Err(protocol_error("expected MAIL FROM"));
    }
    if let Scenario::RejectMail(response) = scenario {
        writer.write_all(response.as_bytes()).await?;
        writer.write_all(b"\r\n").await?;
        return Ok(transcript);
    }
    writer.write_all(b"250 2.1.0 sender accepted\r\n").await?;

    let recipient = read_line(&mut reader).await?;
    transcript.commands.push(recipient.clone());
    if recipient != "RCPT TO:<recipient@example.test>" {
        return Err(protocol_error("expected RCPT TO"));
    }
    writer
        .write_all(b"250 2.1.5 recipient accepted\r\n")
        .await?;

    let data = read_line(&mut reader).await?;
    transcript.commands.push(data.clone());
    if data != "DATA" {
        return Err(protocol_error("expected DATA"));
    }
    writer.write_all(b"354 send message content\r\n").await?;

    loop {
        let line = read_raw_line(&mut reader).await?;
        if line == ".\r\n" {
            break;
        }
        transcript.data.push_str(&line);
    }
    writer.write_all(b"250 2.0.0 queued as fake-42\r\n").await?;
    Ok(transcript)
}

async fn read_line(
    reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
) -> std::io::Result<String> {
    Ok(read_raw_line(reader)
        .await?
        .trim_end_matches(['\r', '\n'])
        .to_string())
}

async fn read_raw_line(
    reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
) -> std::io::Result<String> {
    let mut line = String::new();
    let read = reader.read_line(&mut line).await?;
    if read == 0 {
        return Err(protocol_error("SMTP client closed the connection"));
    }
    Ok(line)
}

fn protocol_error(message: &'static str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message)
}

fn sender(port: u16, credentials: bool, starttls: bool) -> SmtpEmailSender {
    SmtpEmailSender::new(SmtpEmailConfig {
        host: "127.0.0.1".to_string(),
        port,
        username: credentials.then(|| "user".to_string()),
        password: credentials.then(|| "pass".to_string()),
        starttls,
    })
    .expect("valid SMTP sender configuration")
}

fn message() -> EmailMessage {
    EmailMessage {
        from: EmailAddress {
            email: "sender@example.test".to_string(),
            name: Some("Sender".to_string()),
        },
        to: vec![EmailAddress {
            email: "recipient@example.test".to_string(),
            name: None,
        }],
        subject: "Hermetic SMTP integration".to_string(),
        text_body: Some("boundary delivery marker".to_string()),
        html_body: None,
        headers: vec![(
            "X-Nvbes-Email-Job-Id".to_string(),
            "00000000-0000-0000-0000-000000000042".to_string(),
        )],
    }
}

async fn send(sender: &SmtpEmailSender) -> Result<SendResult, EmailError> {
    timeout(TEST_TIMEOUT, sender.send_message(&message()))
        .await
        .expect("SMTP client should finish")
}

#[tokio::test]
async fn smtp_delivers_a_complete_message_over_a_real_tcp_exchange() {
    let (port, server) = spawn_server(Scenario::Delivery).await;

    let result = send(&sender(port, false, false))
        .await
        .expect("SMTP delivery should succeed");
    let transcript = server.await.expect("fake SMTP server task");

    assert_eq!(
        result.provider_email_id,
        "<account-job-00000000-0000-0000-0000-000000000042@notify.nvbes.eu>"
    );
    assert_eq!(transcript.commands.len(), 4);
    assert!(
        transcript
            .data
            .contains("Subject: Hermetic SMTP integration\r\n")
    );
    assert!(transcript.data.contains("boundary delivery marker\r\n"));
}

#[tokio::test]
async fn smtp_authenticates_before_sending_the_envelope() {
    let (port, server) = spawn_server(Scenario::AuthenticatedDelivery).await;

    send(&sender(port, true, false))
        .await
        .expect("authenticated SMTP delivery should succeed");
    let transcript = server.await.expect("fake SMTP server task");

    assert_eq!(transcript.commands[1], "AUTH PLAIN AHVzZXIAcGFzcw==");
    assert!(transcript.commands[2].starts_with("MAIL FROM:"));
}

#[tokio::test]
async fn required_starttls_refuses_a_server_without_the_extension() {
    let (port, server) = spawn_server(Scenario::NoStartTls).await;

    let error = send(&sender(port, false, true))
        .await
        .expect_err("required STARTTLS must reject a plaintext-only server");
    let transcript = server.await.expect("fake SMTP server task");

    assert_eq!(error.safe_code(), "email_smtp_transport");
    assert!(
        transcript
            .commands
            .iter()
            .all(|command| !command.starts_with("MAIL FROM:") && command != "DATA")
    );
}

#[tokio::test]
async fn smtp_classifies_4xx_as_transient_and_5xx_as_permanent() {
    for (response, expected) in [
        (
            "451 4.3.0 temporary local failure",
            EmailFailureClass::Transient,
        ),
        (
            "550 5.1.1 mailbox unavailable",
            EmailFailureClass::Permanent,
        ),
    ] {
        let (port, server) = spawn_server(Scenario::RejectMail(response)).await;

        let error = send(&sender(port, false, false))
            .await
            .expect_err("SMTP rejection should fail delivery");
        server.await.expect("fake SMTP server task");

        assert_eq!(error.failure_class(), expected, "response: {response}");
        assert_eq!(
            error.is_retryable(),
            expected == EmailFailureClass::Transient
        );
    }
}
