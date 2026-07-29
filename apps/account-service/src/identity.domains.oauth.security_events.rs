use std::time::Duration;

use reqwest::StatusCode;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domains::auth::jwt::JwtService;
use crate::http::error::AppError;

const OIDC_LOGOUT: &str = "oidc_backchannel_logout";
const CAEP_SESSION_REVOKED: &str = "caep_session_revoked";
const RISC_CREDENTIAL_COMPROMISE: &str = "risc_credential_compromise";

struct PendingDelivery {
    id: Uuid,
    attempts: i32,
    client_id: String,
    principal_id: Uuid,
    session_id: Uuid,
    event_kind: String,
    endpoint_uri: String,
}

pub async fn enqueue_session_revoked(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    principal_id: Uuid,
    session_id: Uuid,
) -> Result<(), AppError> {
    let client_ids =
        nvbes_redis::refresh_token::client_ids_for_session(redis, principal_id, session_id)
            .await
            .map_err(|error| {
                AppError::internal("security_event_client_lookup_failed", error.to_string())
            })?;
    if client_ids.is_empty() {
        return Ok(());
    }

    let clients = sqlx::query(
        r#"
        SELECT
            id,
            tenant_id,
            backchannel_logout_uri,
            security_event_receiver_uri
        FROM oauth_clients
        WHERE id = ANY($1)
          AND revoked_at IS NULL
        "#,
    )
    .bind(&client_ids)
    .fetch_all(db)
    .await?;

    for client in clients {
        let client_id: Uuid = client.get("id");
        let tenant_id: Uuid = client.get("tenant_id");
        let backchannel_uri: Option<String> = client.get("backchannel_logout_uri");
        let event_receiver_uri: Option<String> = client.get("security_event_receiver_uri");
        if let Some(endpoint) = backchannel_uri {
            enqueue(
                db,
                tenant_id,
                client_id,
                principal_id,
                session_id,
                OIDC_LOGOUT,
                &endpoint,
            )
            .await?;
        }
        if let Some(endpoint) = event_receiver_uri {
            enqueue(
                db,
                tenant_id,
                client_id,
                principal_id,
                session_id,
                CAEP_SESSION_REVOKED,
                &endpoint,
            )
            .await?;
        }
    }
    Ok(())
}

pub async fn enqueue_all_sessions_revoked(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    principal_id: Uuid,
    except_session_id: Option<Uuid>,
) -> Result<(), AppError> {
    let session_ids = nvbes_redis::session::list_user_sessions(redis, &principal_id.to_string())
        .await
        .map_err(|error| {
            AppError::internal("security_event_session_lookup_failed", error.to_string())
        })?;
    for session_id in session_ids {
        let Ok(session_id) = Uuid::parse_str(&session_id) else {
            continue;
        };
        if Some(session_id) != except_session_id {
            enqueue_session_revoked(db, redis, principal_id, session_id).await?;
        }
    }
    Ok(())
}

pub async fn enqueue_refresh_token_compromise(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    principal_id: Uuid,
    session_id: Uuid,
) -> Result<(), AppError> {
    let client_ids =
        nvbes_redis::refresh_token::client_ids_for_session(redis, principal_id, session_id)
            .await
            .map_err(|error| {
                AppError::internal("security_event_client_lookup_failed", error.to_string())
            })?;
    if client_ids.is_empty() {
        return Ok(());
    }
    let clients = sqlx::query(
        r#"
        SELECT id, tenant_id, security_event_receiver_uri
        FROM oauth_clients
        WHERE id = ANY($1)
          AND revoked_at IS NULL
          AND security_event_receiver_uri IS NOT NULL
        "#,
    )
    .bind(&client_ids)
    .fetch_all(db)
    .await?;
    for client in clients {
        let endpoint: String = client.get("security_event_receiver_uri");
        enqueue(
            db,
            client.get("tenant_id"),
            client.get("id"),
            principal_id,
            session_id,
            RISC_CREDENTIAL_COMPROMISE,
            &endpoint,
        )
        .await?;
    }
    Ok(())
}

async fn enqueue(
    db: &PgPool,
    tenant_id: Uuid,
    oauth_client_id: Uuid,
    principal_id: Uuid,
    session_id: Uuid,
    event_kind: &str,
    endpoint_uri: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO security_event_deliveries (
            tenant_id, oauth_client_id, principal_id, session_id, event_kind, endpoint_uri
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (oauth_client_id, session_id, event_kind) DO NOTHING
        "#,
    )
    .bind(tenant_id)
    .bind(oauth_client_id)
    .bind(principal_id)
    .bind(session_id)
    .bind(event_kind)
    .bind(endpoint_uri)
    .execute(db)
    .await?;
    Ok(())
}

pub fn start_dispatcher(db: PgPool, jwt: JwtService) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            if let Err(error) = dispatch_batch(&db, &jwt).await {
                tracing::warn!(error = ?error, "security event delivery batch failed");
            }
        }
    });
}

async fn dispatch_batch(db: &PgPool, jwt: &JwtService) -> Result<(), AppError> {
    for delivery in claim_batch(db).await? {
        match deliver(jwt, &delivery).await {
            Ok(()) => {
                sqlx::query(
                    "UPDATE security_event_deliveries SET delivered_at = NOW(), last_error = NULL WHERE id = $1",
                )
                .bind(delivery.id)
                .execute(db)
                .await?;
            }
            Err(error) => {
                let exponent = u32::try_from(delivery.attempts.saturating_sub(1))
                    .unwrap_or(0)
                    .min(5);
                let retry_seconds = (30_i64 * 2_i64.pow(exponent)).min(900);
                sqlx::query(
                    r#"
                    UPDATE security_event_deliveries
                    SET last_error = $2,
                        next_attempt_at = NOW() + make_interval(secs => $3)
                    WHERE id = $1
                    "#,
                )
                .bind(delivery.id)
                .bind(error.to_string().chars().take(500).collect::<String>())
                .bind(retry_seconds)
                .execute(db)
                .await?;
            }
        }
    }
    Ok(())
}

async fn claim_batch(db: &PgPool) -> Result<Vec<PendingDelivery>, AppError> {
    let rows = sqlx::query(
        r#"
        WITH claimed AS (
            SELECT id
            FROM security_event_deliveries
            WHERE delivered_at IS NULL
              AND next_attempt_at <= NOW()
              AND attempts < 10
            ORDER BY created_at
            FOR UPDATE SKIP LOCKED
            LIMIT 50
        )
        UPDATE security_event_deliveries delivery
        SET attempts = delivery.attempts + 1,
            next_attempt_at = NOW() + INTERVAL '15 minutes'
        FROM claimed
        WHERE delivery.id = claimed.id
        RETURNING
            delivery.id,
            delivery.attempts,
            delivery.oauth_client_id,
            delivery.principal_id,
            delivery.session_id,
            delivery.event_kind,
            delivery.endpoint_uri
        "#,
    )
    .fetch_all(db)
    .await?;

    let mut deliveries = Vec::with_capacity(rows.len());
    for row in rows {
        let oauth_client_id: Uuid = row.get("oauth_client_id");
        let client_id =
            sqlx::query_scalar::<_, String>("SELECT client_id FROM oauth_clients WHERE id = $1")
                .bind(oauth_client_id)
                .fetch_one(db)
                .await?;
        let Some(session_id) = row.get::<Option<Uuid>, _>("session_id") else {
            continue;
        };
        deliveries.push(PendingDelivery {
            id: row.get("id"),
            attempts: row.get("attempts"),
            client_id,
            principal_id: row.get("principal_id"),
            session_id,
            event_kind: row.get("event_kind"),
            endpoint_uri: row.get("endpoint_uri"),
        });
    }
    Ok(deliveries)
}

async fn deliver(jwt: &JwtService, delivery: &PendingDelivery) -> Result<(), String> {
    let (client, destination) = crate::http::outbound::pinned_https_client(
        &delivery.endpoint_uri,
        &[delivery.endpoint_uri.as_str()],
        Duration::from_secs(5),
    )
    .await?;
    let response = match delivery.event_kind.as_str() {
        OIDC_LOGOUT => {
            let token = jwt
                .generate_logout_token(
                    delivery.principal_id,
                    delivery.session_id,
                    &delivery.client_id,
                )
                .await
                .map_err(|error| format!("{error:?}"))?;
            client
                .post(destination.clone())
                .form(&[("logout_token", token)])
                .send()
                .await
                .map_err(|error| error.to_string())?
        }
        CAEP_SESSION_REVOKED => {
            let token = jwt
                .generate_caep_session_revoked_token(
                    delivery.principal_id,
                    delivery.session_id,
                    &delivery.client_id,
                )
                .await
                .map_err(|error| format!("{error:?}"))?;
            client
                .post(destination.clone())
                .header(reqwest::header::CONTENT_TYPE, "application/secevent+jwt")
                .header(reqwest::header::ACCEPT, "application/json")
                .body(token)
                .send()
                .await
                .map_err(|error| error.to_string())?
        }
        RISC_CREDENTIAL_COMPROMISE => {
            let token = jwt
                .generate_risc_credential_compromise_token(
                    delivery.principal_id,
                    &delivery.client_id,
                )
                .await
                .map_err(|error| format!("{error:?}"))?;
            client
                .post(destination)
                .header(reqwest::header::CONTENT_TYPE, "application/secevent+jwt")
                .header(reqwest::header::ACCEPT, "application/json")
                .body(token)
                .send()
                .await
                .map_err(|error| error.to_string())?
        }
        _ => return Err("unsupported security event kind".to_string()),
    };

    let accepted = match delivery.event_kind.as_str() {
        OIDC_LOGOUT => matches!(response.status(), StatusCode::OK | StatusCode::NO_CONTENT),
        CAEP_SESSION_REVOKED | RISC_CREDENTIAL_COMPROMISE => {
            response.status() == StatusCode::ACCEPTED
        }
        _ => false,
    };
    if accepted {
        Ok(())
    } else {
        Err(format!("receiver returned HTTP {}", response.status()))
    }
}

#[cfg(test)]
mod tests {
    use super::{CAEP_SESSION_REVOKED, OIDC_LOGOUT, RISC_CREDENTIAL_COMPROMISE};

    #[test]
    fn event_kinds_are_stable_protocol_identifiers() {
        assert_eq!(OIDC_LOGOUT, "oidc_backchannel_logout");
        assert_eq!(CAEP_SESSION_REVOKED, "caep_session_revoked");
        assert_eq!(RISC_CREDENTIAL_COMPROMISE, "risc_credential_compromise");
    }
}
