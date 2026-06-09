use libxml::tree::Node as XmlNode;
use libxml::xpath::Context;
use roxmltree::Node;
use xmlsec::{XmlSecDocumentExt, XmlSecKey, XmlSecKeyFormat, XmlSecSignatureContext};

use super::types::FederatedIdentityProviderRecord;
use crate::http::error::AppError;

pub fn verify_assertion_signature(
    xml: &str,
    certificates: &[String],
    provider: &FederatedIdentityProviderRecord,
) -> Result<Option<String>, AppError> {
    if !provider.require_signed_assertions {
        return Ok(None);
    }
    verify_signed_element(xml, certificates, "Assertion", "assertion").map(Some)
}

pub fn verify_response_signature_if_required(
    xml: &str,
    certificates: &[String],
    provider: &FederatedIdentityProviderRecord,
) -> Result<(), AppError> {
    if !provider.require_signed_responses {
        return Ok(());
    }
    verify_signed_element(xml, certificates, "Response", "response").map(|_| ())
}

fn verify_signed_element(
    xml: &str,
    certificates: &[String],
    element_name: &str,
    element_type: &str,
) -> Result<String, AppError> {
    let doc = libxml::parser::Parser::default()
        .parse_string(xml)
        .map_err(|err| AppError::bad_request("saml_xml_invalid", format!("{}", err)))?;
    let saml_ns = [
        ("saml", "urn:oasis:names:tc:SAML:2.0:assertion"),
        ("samlp", "urn:oasis:names:tc:SAML:2.0:protocol"),
    ];
    doc.specify_idattr("//saml:Assertion", "ID", Some(&saml_ns))
        .map_err(|err| AppError::bad_request("saml_id_invalid", format!("{}", err)))?;
    let _ = doc.specify_idattr("//samlp:Response", "ID", Some(&saml_ns));

    let mut context = Context::new(&doc).map_err(|_| {
        AppError::internal(
            "saml_xpath_unavailable",
            "Unable to create SAML XML lookup context.",
        )
    })?;

    let elements = context
        .findnodes(&format!("//*[local-name()='{element_name}']"), None)
        .map_err(|_| AppError::bad_request("saml_xml_invalid", "Unable to inspect SAML XML."))?;
    let mut last_error = Some(format!("No signed SAML {element_type} was found."));

    for element in elements {
        let Some(element_id) = element
            .get_property("ID")
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        else {
            continue;
        };

        let signatures = context
            .findnodes("./*[local-name()='Signature']", Some(&element))
            .map_err(|_| {
                AppError::bad_request("saml_xml_invalid", "Unable to inspect SAML signatures.")
            })?;

        for signature in signatures {
            let references = context
                .findnodes(
                    ".//*[local-name()='SignedInfo']/*[local-name()='Reference']",
                    Some(&signature),
                )
                .map_err(|_| {
                    AppError::bad_request(
                        "saml_xml_invalid",
                        "Unable to inspect SAML signature references.",
                    )
                })?;

            let references_element = references.iter().any(|reference| {
                reference
                    .get_property("URI")
                    .as_deref()
                    .is_some_and(|uri| uri == format!("#{element_id}"))
            });
            if !references_element {
                continue;
            }

            match verify_signature_node(&signature, certificates) {
                Ok(()) => return Ok(element_id),
                Err(err) => last_error = Some(err),
            }
        }
    }

    Err(AppError::bad_request(
        "saml_signature_invalid",
        format!(
            "The SAML {} signature could not be verified. {}",
            element_type,
            last_error.unwrap_or_default()
        ),
    ))
}

fn verify_signature_node(signature: &XmlNode, certificates: &[String]) -> Result<(), String> {
    let mut last_error = None;

    for cert in certificates {
        let pem = format!(
            "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----",
            cert
        );

        match verify_xml_signature(signature, pem.as_bytes()) {
            Ok(true) => return Ok(()),
            Ok(false) => last_error = Some("Signature invalid".to_string()),
            Err(err) => last_error = Some(format!("Signature verification error: {}", err)),
        }
    }

    Err(last_error.unwrap_or_else(|| "No signing certificate is configured.".to_string()))
}

fn verify_xml_signature(signature: &XmlNode, cert_pem: &[u8]) -> Result<bool, String> {
    let mut context = XmlSecSignatureContext::new();
    let key = XmlSecKey::from_memory(cert_pem, XmlSecKeyFormat::CertPem, None)
        .map_err(|err| err.to_string())?;
    context.insert_key(key);
    context
        .verify_node(signature)
        .map_err(|err| err.to_string())
}

pub fn validate_assertion_issuer(
    assertion: &Node,
    expected_issuer: &Option<String>,
) -> Result<(), AppError> {
    let assertion_issuer = assertion
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "Issuer")
        .and_then(|n| n.text())
        .map(|t| t.trim());

    if let Some(expected) = expected_issuer
        && assertion_issuer != Some(expected.as_str())
    {
        return Err(AppError::bad_request(
            "saml_issuer_mismatch",
            format!(
                "The SAML assertion issuer '{:?}' does not match the expected issuer '{}'.",
                assertion_issuer, expected
            ),
        ));
    }

    Ok(())
}

pub fn validate_assertion_conditions(
    assertion: &Node,
    expected_audience: &Option<String>,
) -> Result<(), AppError> {
    let Some(conditions) = assertion
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "Conditions")
    else {
        return Ok(());
    };

    let now = chrono::Utc::now();

    if let Some(not_before_str) = conditions.attribute("NotBefore")
        && let Ok(not_before) = chrono::DateTime::parse_from_rfc3339(not_before_str)
        && now < not_before
    {
        return Err(AppError::bad_request(
            "saml_assertion_too_early",
            "The SAML assertion is not yet valid (NotBefore).",
        ));
    }

    if let Some(not_on_or_after_str) = conditions.attribute("NotOnOrAfter")
        && let Ok(not_on_or_after) = chrono::DateTime::parse_from_rfc3339(not_on_or_after_str)
        && now >= not_on_or_after
    {
        return Err(AppError::bad_request(
            "saml_assertion_expired",
            "The SAML assertion has expired (NotOnOrAfter).",
        ));
    }

    if let Some(expected_audience) = expected_audience {
        let audience_restriction = conditions
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "AudienceRestriction");

        if let Some(restriction) = audience_restriction {
            let has_audience = restriction
                .children()
                .filter(|n| n.is_element() && n.tag_name().name() == "Audience")
                .any(|n| n.text().map(|t| t.trim()) == Some(expected_audience));

            if !has_audience {
                return Err(AppError::bad_request(
                    "saml_audience_mismatch",
                    format!(
                        "The SAML assertion audience does not match the expected audience '{}'.",
                        expected_audience
                    ),
                ));
            }
        }
    }

    Ok(())
}

pub fn extract_attributes(
    assertion: &Node,
    attribute_mapping: &serde_json::Value,
) -> (Option<String>, Option<String>) {
    let mut email = None;
    let mut username = None;

    let Some(attribute_statement) = assertion
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "AttributeStatement")
    else {
        return (email, username);
    };

    for attribute in attribute_statement
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "Attribute")
    {
        let attr_name = attribute.attribute("Name").unwrap_or_default();
        let value = attribute
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "AttributeValue")
            .and_then(|n| n.text())
            .map(|t| t.trim().to_string());

        let mapped_name = map_attribute_name(attr_name, attribute_mapping);

        match mapped_name.as_str() {
            "email" if email.is_none() => email = value,
            "username" if username.is_none() => username = value,
            _ => {}
        }
    }

    (email, username)
}

fn map_attribute_name(raw: &str, mapping: &serde_json::Value) -> String {
    if let Some(mapped) = mapping.get(raw).and_then(|v| v.as_str()) {
        return mapped.to_string();
    }

    match raw {
        "email"
        | "mail"
        | "Email"
        | "EmailAddress"
        | "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/emailaddress" => {
            "email".to_string()
        }
        "username"
        | "preferred_username"
        | "uid"
        | "upn"
        | "sAMAccountName"
        | "samaccountname"
        | "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/upn" => "username".to_string(),
        _ => raw.to_string(),
    }
}
