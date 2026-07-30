use base64::Engine;
use sqlx::Row;
use tonic::{Request, Status};
use uuid::Uuid;

use crate::database::Database;

const BASIC_PREFIX: &str = "Basic ";

pub async fn authenticate_client<T>(db: &Database, request: &Request<T>) -> Result<String, Status> {
    let (client_id, client_secret) = parse_basic_credentials(request)?;
    let row = sqlx::query(
        r#"
        SELECT client_secret_hash, tenant_id, client_assertion_required, revoked_at
        FROM oauth_clients
        WHERE client_id = $1
        LIMIT 1
        "#,
    )
    .bind(&client_id)
    .fetch_optional(db)
    .await
    .map_err(|error| {
        tracing::error!(?error, "identity gRPC client lookup failed");
        Status::internal("Identity client authentication failed.")
    })?
    .ok_or_else(invalid_client)?;

    if row
        .get::<Option<chrono::DateTime<chrono::Utc>>, _>("revoked_at")
        .is_some()
        || row.get::<bool, _>("client_assertion_required")
    {
        return Err(invalid_client());
    }

    let tenant_id: Uuid = row.get("tenant_id");
    let secret_hash: String = row.get("client_secret_hash");
    crate::domains::oauth::verify_client_secret_with_overlap(
        tenant_id,
        &client_id,
        &client_secret,
        &secret_hash,
    )
    .await
    .map_err(|_| invalid_client())?;

    Ok(client_id)
}

fn parse_basic_credentials<T>(request: &Request<T>) -> Result<(String, String), Status> {
    let encoded = request
        .metadata()
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix(BASIC_PREFIX))
        .ok_or_else(invalid_client)?;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .ok_or_else(invalid_client)?;
    let (client_id, client_secret) = decoded.split_once(':').ok_or_else(invalid_client)?;
    let client_id = client_id.trim();
    if client_id.is_empty() || client_secret.is_empty() {
        return Err(invalid_client());
    }
    Ok((client_id.to_string(), client_secret.to_string()))
}

fn invalid_client() -> Status {
    Status::unauthenticated("Invalid internal Identity client credential.")
}

#[cfg(test)]
mod tests {
    use base64::Engine;
    use tonic::Request;

    use super::parse_basic_credentials;

    #[test]
    fn parses_basic_client_credentials() {
        let encoded =
            base64::engine::general_purpose::STANDARD.encode("cloud-service:client-secret");
        let mut request = Request::new(());
        request.metadata_mut().insert(
            "authorization",
            format!("Basic {encoded}")
                .parse()
                .expect("metadata must parse"),
        );

        assert_eq!(
            parse_basic_credentials(&request).unwrap(),
            ("cloud-service".to_string(), "client-secret".to_string())
        );
    }

    #[test]
    fn rejects_missing_credentials() {
        assert!(parse_basic_credentials(&Request::new(())).is_err());
    }
}
