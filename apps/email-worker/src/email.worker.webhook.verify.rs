use std::{collections::HashMap, time::Duration};

use base64::{Engine, engine::general_purpose::STANDARD};
use chrono::{DateTime, Utc};
use openssl::{
    hash::MessageDigest,
    sign::Verifier,
    stack::Stack,
    x509::{X509, X509StoreContext, store::X509StoreBuilder},
};
use reqwest::{Client, Url, redirect::Policy};
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::config::WebhookTrustConfig;

const MAX_MESSAGE_AGE: chrono::Duration = chrono::Duration::minutes(5);
const MAX_CERTIFICATE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SnsMessage {
    #[serde(rename = "Type")]
    pub message_type: String,
    #[serde(rename = "MessageId")]
    pub message_id: String,
    #[serde(rename = "TopicArn")]
    pub topic_arn: String,
    pub message: String,
    pub timestamp: String,
    pub signature_version: String,
    pub signature: String,
    #[serde(rename = "SigningCertURL")]
    pub signing_cert_url: String,
    pub subject: Option<String>,
    pub token: Option<String>,
    #[serde(rename = "SubscribeURL")]
    pub subscribe_url: Option<String>,
}

pub struct WebhookVerifier {
    topic_arn: String,
    signing_certificate_host: String,
    confirmation_host: String,
    trust_chain: Vec<X509>,
    client: Client,
    certificates: RwLock<HashMap<String, X509>>,
}

impl WebhookVerifier {
    pub fn new(config: &WebhookTrustConfig) -> anyhow::Result<Self> {
        let trust_chain = X509::stack_from_pem(&config.ca_bundle_pem)?;
        if trust_chain.is_empty() {
            anyhow::bail!("SNS CA bundle contains no certificate");
        }
        let client = Client::builder()
            .https_only(true)
            .redirect(Policy::none())
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(5))
            .build()?;
        Ok(Self {
            topic_arn: config.topic_arn.clone(),
            signing_certificate_host: config.signing_certificate_host.clone(),
            confirmation_host: config.confirmation_host.clone(),
            trust_chain,
            client,
            certificates: RwLock::new(HashMap::new()),
        })
    }

    pub async fn verify(&self, body: &[u8]) -> anyhow::Result<SnsMessage> {
        let message: SnsMessage = serde_json::from_slice(body)?;
        self.validate_envelope(&message)?;
        let certificate_url = validated_url(
            &message.signing_cert_url,
            &self.signing_certificate_host,
            "/fr-par/sns/",
        )?;
        let certificate = self.certificate(&certificate_url).await?;
        let signature = STANDARD.decode(&message.signature)?;
        let public_key = certificate.public_key()?;
        let mut verifier = Verifier::new(MessageDigest::sha1(), &public_key)?;
        verifier.update(canonical_message(&message)?.as_bytes())?;
        if !verifier.verify(&signature)? {
            anyhow::bail!("SNS signature is invalid");
        }
        Ok(message)
    }

    pub fn confirmation_url(&self, message: &SnsMessage) -> anyhow::Result<Url> {
        let value = message
            .subscribe_url
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("SNS confirmation URL is missing"))?;
        let url = Url::parse(value)?;
        if url.scheme() != "https"
            || url.host_str() != Some(self.confirmation_host.as_str())
            || url.port().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
        {
            anyhow::bail!("SNS confirmation URL is not allowed");
        }
        let query: HashMap<_, _> = url.query_pairs().into_owned().collect();
        if query.get("Action").map(String::as_str) != Some("ConfirmSubscription")
            || query.get("TopicArn") != Some(&message.topic_arn)
            || query.get("Token").map(String::as_str) != message.token.as_deref()
        {
            anyhow::bail!("SNS confirmation URL parameters are invalid");
        }
        Ok(url)
    }

    pub async fn confirm(&self, url: Url) -> anyhow::Result<()> {
        let response = self.client.get(url).send().await?;
        if !response.status().is_success() {
            anyhow::bail!("SNS subscription confirmation was rejected");
        }
        Ok(())
    }

    fn validate_envelope(&self, message: &SnsMessage) -> anyhow::Result<()> {
        if message.signature_version != "1" {
            anyhow::bail!("unsupported SNS signature version");
        }
        if message.topic_arn != self.topic_arn {
            anyhow::bail!("SNS topic is not allowed");
        }
        if !matches!(
            message.message_type.as_str(),
            "Notification" | "SubscriptionConfirmation"
        ) {
            anyhow::bail!("unsupported SNS message type");
        }
        let timestamp = DateTime::parse_from_rfc3339(&message.timestamp)?.with_timezone(&Utc);
        let age = Utc::now() - timestamp;
        if age < -MAX_MESSAGE_AGE || age > MAX_MESSAGE_AGE {
            anyhow::bail!("SNS message timestamp is stale");
        }
        Ok(())
    }

    async fn certificate(&self, url: &Url) -> anyhow::Result<X509> {
        if let Some(certificate) = self.certificates.read().await.get(url.as_str()).cloned() {
            return Ok(certificate);
        }
        let response = self.client.get(url.clone()).send().await?;
        if !response.status().is_success()
            || response.content_length().unwrap_or(0) > MAX_CERTIFICATE_BYTES as u64
        {
            anyhow::bail!("SNS signing certificate could not be loaded");
        }
        let pem = response.bytes().await?;
        if pem.len() > MAX_CERTIFICATE_BYTES {
            anyhow::bail!("SNS signing certificate is too large");
        }
        let certificate = X509::from_pem(&pem)?;
        self.validate_certificate(&certificate)?;
        self.certificates
            .write()
            .await
            .insert(url.to_string(), certificate.clone());
        Ok(certificate)
    }

    fn validate_certificate(&self, certificate: &X509) -> anyhow::Result<()> {
        let mut store = X509StoreBuilder::new()?;
        for authority in &self.trust_chain {
            store.add_cert(authority.clone())?;
        }
        let store = store.build();
        let chain = Stack::new()?;
        let mut context = X509StoreContext::new()?;
        if !context.init(&store, certificate, &chain, |context| context.verify_cert())? {
            anyhow::bail!("SNS signing certificate is not trusted");
        }
        Ok(())
    }
}

fn validated_url(value: &str, expected_host: &str, path_prefix: &str) -> anyhow::Result<Url> {
    let url = Url::parse(value)?;
    if url.scheme() != "https"
        || url.host_str() != Some(expected_host)
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.path().starts_with(path_prefix)
    {
        anyhow::bail!("SNS URL is not allowed");
    }
    Ok(url)
}

fn canonical_message(message: &SnsMessage) -> anyhow::Result<String> {
    let fields: Vec<(&str, &str)> = match message.message_type.as_str() {
        "Notification" => {
            let mut fields = vec![
                ("Message", message.message.as_str()),
                ("MessageId", message.message_id.as_str()),
            ];
            if let Some(subject) = message.subject.as_deref() {
                fields.push(("Subject", subject));
            }
            fields.extend([
                ("Timestamp", message.timestamp.as_str()),
                ("TopicArn", message.topic_arn.as_str()),
                ("Type", message.message_type.as_str()),
            ]);
            fields
        }
        "SubscriptionConfirmation" => vec![
            ("Message", message.message.as_str()),
            ("MessageId", message.message_id.as_str()),
            (
                "SubscribeURL",
                message
                    .subscribe_url
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("SubscribeURL is missing"))?,
            ),
            ("Timestamp", message.timestamp.as_str()),
            (
                "Token",
                message
                    .token
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("Token is missing"))?,
            ),
            ("TopicArn", message.topic_arn.as_str()),
            ("Type", message.message_type.as_str()),
        ],
        _ => anyhow::bail!("unsupported SNS message type"),
    };
    Ok(fields
        .into_iter()
        .map(|(name, value)| format!("{name}\n{value}\n"))
        .collect())
}

#[cfg(test)]
#[path = "email.worker.webhook.verify.tests.rs"]
mod tests;
