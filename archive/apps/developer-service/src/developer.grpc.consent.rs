use chrono::{DateTime, Utc};
use sqlx::{Row, postgres::PgRow};
use tonic::Status;
use url::Url;
use uuid::Uuid;

use crate::grpc::{
    pb::nvbes::developer::v1 as developer,
    service_status::{non_empty, parse_uuid, sql_status},
};

pub async fn get_consent_screen(
    db: &sqlx::PgPool,
    request: developer::GetConsentScreenRequest,
) -> Result<developer::ConsentScreen, Status> {
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let client_id = non_empty(request.client_id, "client_id")?;
    get_consent_screen_record(db, tenant_id, &client_id).await
}

pub async fn get_public_consent_screen(
    db: &sqlx::PgPool,
    request: developer::GetPublicConsentScreenRequest,
) -> Result<developer::ConsentScreen, Status> {
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let client_id = non_empty(request.client_id, "client_id")?;
    get_consent_screen_record(db, tenant_id, &client_id).await
}

pub async fn upsert_consent_screen(
    db: &sqlx::PgPool,
    request: developer::UpsertConsentScreenRequest,
) -> Result<developer::ConsentScreen, Status> {
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let actor_id = actor_id(&request.context)?;
    let client_id = non_empty(request.client_id, "client_id")?;
    let logo_url = https_url_option(request.logo_url, "logo_url")?;
    let support_url = https_url_option(request.support_url, "support_url")?;
    let privacy_url = https_url_option(request.privacy_url, "privacy_url")?;
    let terms_url = https_url_option(request.terms_url, "terms_url")?;
    let custom_css = safe_css_option(request.custom_css)?;

    sqlx::query(
        r#"
        INSERT INTO developer_consent_screens (
          tenant_id,
          client_id,
          product_name,
          logo_url,
          support_url,
          privacy_url,
          terms_url,
          description,
          brand_color,
          custom_css,
          help_text,
          updated_by,
          updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, now())
        ON CONFLICT (tenant_id, client_id) DO UPDATE
        SET product_name = EXCLUDED.product_name,
            logo_url = EXCLUDED.logo_url,
            support_url = EXCLUDED.support_url,
            privacy_url = EXCLUDED.privacy_url,
            terms_url = EXCLUDED.terms_url,
            description = EXCLUDED.description,
            brand_color = EXCLUDED.brand_color,
            custom_css = EXCLUDED.custom_css,
            help_text = EXCLUDED.help_text,
            updated_by = EXCLUDED.updated_by,
            updated_at = now()
        "#,
    )
    .bind(tenant_id)
    .bind(&client_id)
    .bind(request.product_name)
    .bind(logo_url)
    .bind(support_url)
    .bind(privacy_url)
    .bind(terms_url)
    .bind(request.description)
    .bind(empty_to_none(request.brand_color))
    .bind(custom_css)
    .bind(empty_to_none(request.help_text))
    .bind(actor_id)
    .execute(db)
    .await
    .map_err(sql_status)?;

    get_consent_screen_record(db, tenant_id, &client_id).await
}

async fn get_consent_screen_record(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    client_id: &str,
) -> Result<developer::ConsentScreen, Status> {
    let screen = sqlx::query(
        r#"
        SELECT
          client_id,
          product_name,
          logo_url,
          support_url,
          privacy_url,
          terms_url,
          description,
          brand_color,
          custom_css,
          help_text,
          true AS configured,
          updated_at
        FROM developer_consent_screens
        WHERE tenant_id = $1
          AND client_id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(client_id)
    .fetch_optional(db)
    .await
    .map_err(sql_status)?;

    Ok(screen
        .map(consent_screen_from_row)
        .unwrap_or_else(|| empty_consent_screen(client_id)))
}

fn consent_screen_from_row(row: PgRow) -> developer::ConsentScreen {
    developer::ConsentScreen {
        client_id: row.get("client_id"),
        product_name: row.get("product_name"),
        logo_url: row.get::<Option<String>, _>("logo_url").unwrap_or_default(),
        support_url: row
            .get::<Option<String>, _>("support_url")
            .unwrap_or_default(),
        privacy_url: row
            .get::<Option<String>, _>("privacy_url")
            .unwrap_or_default(),
        terms_url: row
            .get::<Option<String>, _>("terms_url")
            .unwrap_or_default(),
        description: row.get("description"),
        brand_color: row
            .get::<Option<String>, _>("brand_color")
            .unwrap_or_default(),
        custom_css: row
            .get::<Option<String>, _>("custom_css")
            .unwrap_or_default(),
        help_text: row
            .get::<Option<String>, _>("help_text")
            .unwrap_or_default(),
        configured: row.get("configured"),
        updated_at: row
            .get::<Option<DateTime<Utc>>, _>("updated_at")
            .map(time_string)
            .unwrap_or_default(),
    }
}

fn empty_consent_screen(client_id: &str) -> developer::ConsentScreen {
    developer::ConsentScreen {
        client_id: client_id.to_string(),
        product_name: String::new(),
        logo_url: String::new(),
        support_url: String::new(),
        privacy_url: String::new(),
        terms_url: String::new(),
        description: String::new(),
        brand_color: String::new(),
        custom_css: String::new(),
        help_text: String::new(),
        configured: false,
        updated_at: String::new(),
    }
}

fn actor_id(
    context: &Option<crate::grpc::pb::nvbes::platform::v1::RequestContext>,
) -> Result<Uuid, Status> {
    let context = context
        .as_ref()
        .ok_or_else(|| Status::invalid_argument("request context is required"))?;
    parse_uuid(&context.actor_principal_id, "actor_principal_id")
}

fn empty_to_none(value: String) -> Option<String> {
    let value = value.trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

fn https_url_option(value: String, field: &str) -> Result<Option<String>, Status> {
    let Some(value) = empty_to_none(value) else {
        return Ok(None);
    };
    if value.len() > 2048 {
        return Err(Status::invalid_argument(format!("{field} is too long")));
    }
    let url =
        Url::parse(&value).map_err(|_| Status::invalid_argument(format!("{field} must be URL")))?;
    if url.scheme() != "https" || url.username() != "" || url.password().is_some() {
        return Err(Status::invalid_argument(format!(
            "{field} must be an HTTPS URL without credentials"
        )));
    }
    Ok(Some(value))
}

fn safe_css_option(value: String) -> Result<Option<String>, Status> {
    let Some(value) = empty_to_none(value) else {
        return Ok(None);
    };
    let lower = value.to_lowercase();
    let unsafe_css = [
        "</style",
        "@import",
        "url(",
        "expression(",
        "behavior:",
        "-moz-binding:",
        "javascript:",
        "vbscript:",
        "data:",
    ];
    if unsafe_css.iter().any(|needle| lower.contains(needle)) {
        return Err(Status::invalid_argument("custom_css contains unsafe CSS"));
    }
    Ok(Some(value))
}

fn time_string(value: DateTime<Utc>) -> String {
    value.to_rfc3339()
}

#[cfg(test)]
mod tests {
    #[test]
    fn https_url_option_rejects_script_and_cleartext_urls() {
        assert!(super::https_url_option("javascript:alert(1)".to_string(), "logo_url").is_err());
        assert!(
            super::https_url_option("http://example.test/logo.png".to_string(), "logo_url")
                .is_err()
        );
        assert!(
            super::https_url_option("https://user@example.test/logo.png".to_string(), "logo_url")
                .is_err()
        );
        assert!(
            super::https_url_option("https://example.test/logo.png".to_string(), "logo_url")
                .expect("https URL should be accepted")
                .is_some()
        );
    }

    #[test]
    fn safe_css_option_rejects_external_loads_and_style_breakout() {
        assert!(super::safe_css_option("button { color: red; }".to_string()).is_ok());
        assert!(
            super::safe_css_option("@import url(https://evil.test/x.css);".to_string()).is_err()
        );
        assert!(super::safe_css_option("</style><script>alert(1)</script>".to_string()).is_err());
        assert!(
            super::safe_css_option("body { background: url(https://evil.test/x); }".to_string())
                .is_err()
        );
    }
}
