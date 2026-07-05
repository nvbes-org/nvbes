use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    http::fetch_federation_url,
    saml_replay, saml_subject, saml_validation,
    types::{FederatedIdentityProviderRecord, SamlMetadataResponse},
};
use crate::http::error::AppError;

#[derive(Debug)]
pub struct SamlResponseClaims {
    pub name_id: String,
    pub email: Option<String>,
    pub username: Option<String>,
    pub in_response_to: Option<String>,
}

impl SamlResponseClaims {
    pub fn username(&self) -> String {
        self.username
            .clone()
            .unwrap_or_else(|| self.name_id.clone())
    }
}

pub async fn fetch_saml_metadata(
    provider: &FederatedIdentityProviderRecord,
    strict_mode: bool,
) -> Result<SamlMetadataResponse, AppError> {
    let metadata_url = provider.metadata_url.as_ref().ok_or_else(|| {
        AppError::bad_request(
            "metadata_missing",
            "The SAML provider is missing a metadata_url.",
        )
    })?;
    let response = fetch_federation_url(metadata_url, strict_mode).await?;
    let xml = response.text().await.map_err(|err| {
        AppError::bad_request(
            crate::domains::federation::contract::SAML_METADATA_INVALID,
            format!("{}", err),
        )
    })?;
    let doc = roxmltree::Document::parse(&xml).map_err(|err| {
        AppError::bad_request(
            crate::domains::federation::contract::SAML_METADATA_INVALID,
            format!("{}", err),
        )
    })?;
    let root = doc.root_element();
    let entity_id = root
        .attribute("entityID")
        .or_else(|| root.attribute("entityId"))
        .or_else(|| root.attribute("entity_id"))
        .ok_or_else(|| {
            AppError::bad_request(
                crate::domains::federation::contract::SAML_METADATA_INVALID,
                "The metadata document is missing an entityID.",
            )
        })?
        .to_string();
    let mut signing_certificates = Vec::new();
    let mut sso_url = None;
    for node in doc.descendants() {
        if node.is_element() && node.tag_name().name() == "SingleSignOnService" && sso_url.is_none()
        {
            sso_url = node.attribute("Location").map(|value| value.to_string());
        }
        if node.is_element()
            && node.tag_name().name() == "X509Certificate"
            && node
                .ancestors()
                .any(|ancestor| ancestor.tag_name().name() == "KeyDescriptor")
            && let Some(text) = node.text()
        {
            let value = text.trim().replace('\n', "");
            if !value.is_empty() {
                signing_certificates.push(value);
            }
        }
    }
    Ok(SamlMetadataResponse {
        entity_id,
        sso_url,
        signing_certificates,
        supports_signed_assertions: true,
        supports_signed_responses: true,
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "SAML response validation keeps tenant, provider, recipient, and transport context explicit."
)]
pub async fn validate_saml_response(
    db: &PgPool,
    tenant_id: Uuid,
    provider_id: Uuid,
    provider: &FederatedIdentityProviderRecord,
    saml_response: &str,
    relay_state: Option<&str>,
    strict_mode: bool,
    expected_recipient: &str,
) -> Result<SamlResponseClaims, AppError> {
    let metadata = fetch_saml_metadata(provider, strict_mode).await?;
    let decoded = URL_SAFE_NO_PAD
        .decode(saml_response.as_bytes())
        .or_else(|_| base64::engine::general_purpose::STANDARD.decode(saml_response.as_bytes()))
        .map_err(|err| {
            AppError::bad_request(
                crate::domains::federation::contract::INVALID_SAML_RESPONSE,
                format!("{}", err),
            )
        })?;
    let xml = String::from_utf8(decoded).map_err(|err| {
        AppError::bad_request(
            crate::domains::federation::contract::INVALID_SAML_RESPONSE,
            format!("{}", err),
        )
    })?;
    let doc = roxmltree::Document::parse(&xml).map_err(|err| {
        AppError::bad_request(
            crate::domains::federation::contract::INVALID_SAML_RESPONSE,
            format!("{}", err),
        )
    })?;

    if relay_state.is_some_and(|value| value.trim().is_empty()) {
        return Err(AppError::bad_request(
            crate::domains::federation::contract::INVALID_SAML_RESPONSE,
            "The relay state cannot be empty.",
        ));
    }

    let signed_assertion_id = saml_validation::verify_assertion_signature(
        &xml,
        &metadata.signing_certificates,
        provider,
    )?;
    saml_validation::verify_response_signature_if_required(
        &xml,
        &metadata.signing_certificates,
        provider,
    )?;

    let assertion = match signed_assertion_id.as_deref() {
        Some(assertion_id) => find_assertion_by_id(&doc, assertion_id)?,
        None => first_assertion(&doc)?,
    };
    let assertion_id = assertion
        .attribute("ID")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::bad_request(
                crate::domains::federation::contract::INVALID_SAML_RESPONSE,
                "The SAML assertion is missing an ID attribute.",
            )
        })?;

    saml_validation::validate_assertion_issuer(&assertion, &provider.issuer)?;
    saml_validation::validate_assertion_conditions(&assertion, &provider.client_id)?;

    let subject_data = saml_subject::validate_subject_confirmation(&assertion, expected_recipient)?;

    if let Some(ref in_response_to) = subject_data.in_response_to {
        validate_in_response_to(db, tenant_id, provider_id, in_response_to).await?;
    }

    saml_replay::validate_and_record_assertion(db, tenant_id, provider_id, &assertion_id).await?;

    let name_id = assertion
        .descendants()
        .find(|node| node.is_element() && node.tag_name().name() == "NameID")
        .and_then(|node| node.text())
        .map(|value| value.trim().to_string())
        .ok_or_else(|| {
            AppError::bad_request(
                crate::domains::federation::contract::INVALID_SAML_RESPONSE,
                "The SAML assertion is missing a NameID.",
            )
        })?;

    let (email, username) =
        saml_validation::extract_attributes(&assertion, &provider.attribute_mapping);

    Ok(SamlResponseClaims {
        name_id,
        email,
        username,
        in_response_to: subject_data.in_response_to,
    })
}

fn first_assertion<'a, 'input>(
    doc: &'a roxmltree::Document<'input>,
) -> Result<roxmltree::Node<'a, 'input>, AppError> {
    let mut assertions = doc
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "Assertion");
    let assertion = assertions.next().ok_or_else(|| {
        AppError::bad_request(
            crate::domains::federation::contract::INVALID_SAML_RESPONSE,
            "The SAML response is missing an assertion.",
        )
    })?;

    if assertions.next().is_some() {
        return Err(AppError::bad_request(
            crate::domains::federation::contract::INVALID_SAML_RESPONSE,
            "Unsigned SAML responses with multiple assertions are not accepted.",
        ));
    }

    Ok(assertion)
}

fn find_assertion_by_id<'a, 'input>(
    doc: &'a roxmltree::Document<'input>,
    assertion_id: &str,
) -> Result<roxmltree::Node<'a, 'input>, AppError> {
    let mut matches = doc.descendants().filter(|node| {
        node.is_element()
            && node.tag_name().name() == "Assertion"
            && node.attribute("ID") == Some(assertion_id)
    });
    let assertion = matches.next().ok_or_else(|| {
        AppError::bad_request(
            crate::domains::federation::contract::INVALID_SAML_RESPONSE,
            "The signed SAML assertion was not found.",
        )
    })?;

    if matches.next().is_some() {
        return Err(AppError::bad_request(
            crate::domains::federation::contract::INVALID_SAML_RESPONSE,
            "The SAML response contains duplicate assertion IDs.",
        ));
    }

    Ok(assertion)
}

async fn validate_in_response_to(
    db: &PgPool,
    tenant_id: Uuid,
    provider_id: Uuid,
    request_id: &str,
) -> Result<(), AppError> {
    let parsed_id = Uuid::parse_str(request_id).map_err(|_| {
        AppError::bad_request(
            "saml_invalid_request_id",
            "The InResponseTo value is not a valid request ID.",
        )
    })?;

    let row: Option<(Uuid, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        "SELECT id, expires_at FROM saml_pending_requests WHERE id = $1 AND tenant_id = $2 AND provider_id = $3",
    )
    .bind(parsed_id)
    .bind(tenant_id)
    .bind(provider_id)
    .fetch_optional(db)
    .await?;

    let (row_id, expires_at) = row.ok_or_else(|| {
        AppError::bad_request(
            "saml_unexpected_in_response_to",
            "The InResponseTo value does not match any pending authentication request.",
        )
    })?;

    if chrono::Utc::now() >= expires_at {
        sqlx::query("DELETE FROM saml_pending_requests WHERE id = $1")
            .bind(row_id)
            .execute(db)
            .await?;

        return Err(AppError::bad_request(
            "saml_request_expired",
            "The authentication request has expired.",
        ));
    }

    sqlx::query("DELETE FROM saml_pending_requests WHERE id = $1")
        .bind(row_id)
        .execute(db)
        .await?;

    Ok(())
}

#[cfg(test)]
#[path = "identity.domains.federation.saml.tests.rs"]
mod tests;
