use sqlx::{PgPool, Row};
use uuid::Uuid;

use nvbes_redis::session::CachedSession;

pub async fn insert_session_db(db: &PgPool, session: &CachedSession) -> Result<(), sqlx::Error> {
    let session_id = Uuid::parse_str(&session.session_id).unwrap_or_default();
    let principal_id = Uuid::parse_str(&session.principal_id).unwrap_or_default();
    let tenant_id = session
        .tenant_id
        .as_deref()
        .and_then(|s| Uuid::parse_str(s).ok());
    let organization_id = session
        .organization_id
        .as_deref()
        .and_then(|s| Uuid::parse_str(s).ok());
    let workspace_id = session
        .workspace_id
        .as_deref()
        .and_then(|s| Uuid::parse_str(s).ok());
    let account_device_id = session
        .account_device_id
        .as_deref()
        .and_then(|s| Uuid::parse_str(s).ok());
    let amr_json = serde_json::to_value(&session.amr).unwrap_or_else(|_| serde_json::json!([]));

    sqlx::query(
        r#"
        INSERT INTO user_sessions (
            session_id, principal_id, browser_session_token_hash,
            tenant_id, organization_id, workspace_id, workspace_region,
            client_id, acr, amr, auth_time, idle_timeout_seconds, idle_expires_at,
            expires_at, step_up_verified_at, step_up_expires_at, revoked_at,
            ip, user_agent, cookie_theft_risk_score, cookie_theft_detected_at,
            account_device_id, device_trust_level, device_trust_score,
            risk_score, risk_decision, activity_window_started_at,
            activity_request_count, last_activity_risk_event_at, last_seen_at, created_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15,
            $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31
        )
        ON CONFLICT (session_id) DO UPDATE SET
            browser_session_token_hash = EXCLUDED.browser_session_token_hash,
            tenant_id = EXCLUDED.tenant_id,
            organization_id = EXCLUDED.organization_id,
            workspace_id = EXCLUDED.workspace_id,
            workspace_region = EXCLUDED.workspace_region,
            client_id = EXCLUDED.client_id,
            acr = EXCLUDED.acr,
            amr = EXCLUDED.amr,
            idle_expires_at = EXCLUDED.idle_expires_at,
            expires_at = EXCLUDED.expires_at,
            step_up_verified_at = EXCLUDED.step_up_verified_at,
            step_up_expires_at = EXCLUDED.step_up_expires_at,
            revoked_at = EXCLUDED.revoked_at,
            ip = EXCLUDED.ip,
            user_agent = EXCLUDED.user_agent,
            cookie_theft_risk_score = EXCLUDED.cookie_theft_risk_score,
            cookie_theft_detected_at = EXCLUDED.cookie_theft_detected_at,
            account_device_id = EXCLUDED.account_device_id,
            device_trust_level = EXCLUDED.device_trust_level,
            device_trust_score = EXCLUDED.device_trust_score,
            risk_score = EXCLUDED.risk_score,
            risk_decision = EXCLUDED.risk_decision,
            activity_window_started_at = EXCLUDED.activity_window_started_at,
            activity_request_count = EXCLUDED.activity_request_count,
            last_activity_risk_event_at = EXCLUDED.last_activity_risk_event_at,
            last_seen_at = EXCLUDED.last_seen_at
        "#,
    )
    .bind(session_id)
    .bind(principal_id)
    .bind(&session.browser_session_token_hash)
    .bind(tenant_id)
    .bind(organization_id)
    .bind(workspace_id)
    .bind(&session.workspace_region)
    .bind(&session.client_id)
    .bind(&session.acr)
    .bind(amr_json)
    .bind(session.auth_time)
    .bind(session.idle_timeout_seconds)
    .bind(session.idle_expires_at)
    .bind(session.expires_at)
    .bind(session.step_up_verified_at)
    .bind(session.step_up_expires_at)
    .bind(session.revoked_at)
    .bind(&session.ip)
    .bind(&session.user_agent)
    .bind(session.cookie_theft_risk_score)
    .bind(session.cookie_theft_detected_at)
    .bind(account_device_id)
    .bind(&session.device_trust_level)
    .bind(session.device_trust_score)
    .bind(session.risk_score)
    .bind(&session.risk_decision)
    .bind(session.activity_window_started_at)
    .bind(session.activity_request_count as i32)
    .bind(session.last_activity_risk_event_at)
    .bind(session.last_seen_at)
    .bind(session.created_at)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn fetch_session_db(
    db: &PgPool,
    session_id: Uuid,
) -> Result<Option<CachedSession>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT
            session_id, principal_id, browser_session_token_hash,
            tenant_id, organization_id, workspace_id, workspace_region,
            client_id, acr, amr, auth_time, idle_timeout_seconds, idle_expires_at,
            expires_at, step_up_verified_at, step_up_expires_at, revoked_at,
            ip, user_agent, cookie_theft_risk_score, cookie_theft_detected_at,
            account_device_id, device_trust_level, device_trust_score,
            risk_score, risk_decision, activity_window_started_at,
            activity_request_count, last_activity_risk_event_at, last_seen_at, created_at
        FROM user_sessions
        WHERE session_id = $1 AND revoked_at IS NULL AND expires_at > NOW()
        "#,
    )
    .bind(session_id)
    .fetch_optional(db)
    .await?;

    let row = match row {
        Some(r) => r,
        None => return Ok(None),
    };

    let session_id_val: Uuid = row.get("session_id");
    let principal_id_val: Uuid = row.get("principal_id");
    let tenant_id_val: Option<Uuid> = row.get("tenant_id");
    let organization_id_val: Option<Uuid> = row.get("organization_id");
    let workspace_id_val: Option<Uuid> = row.get("workspace_id");
    let account_device_id_val: Option<Uuid> = row.get("account_device_id");
    let amr_val: serde_json::Value = row.get("amr");
    let amr: Vec<String> = serde_json::from_value(amr_val).unwrap_or_default();
    let req_count: i32 = row.get("activity_request_count");

    Ok(Some(CachedSession {
        session_id: session_id_val.to_string(),
        principal_id: principal_id_val.to_string(),
        browser_session_token_hash: row.get("browser_session_token_hash"),
        tenant_id: tenant_id_val.map(|u| u.to_string()),
        organization_id: organization_id_val.map(|u| u.to_string()),
        workspace_id: workspace_id_val.map(|u| u.to_string()),
        workspace_region: row.get("workspace_region"),
        client_id: row.get("client_id"),
        acr: row.get("acr"),
        amr,
        auth_time: row.get("auth_time"),
        created_at: row.get("created_at"),
        last_seen_at: row.get("last_seen_at"),
        idle_timeout_seconds: row.get("idle_timeout_seconds"),
        idle_expires_at: row.get("idle_expires_at"),
        expires_at: row.get("expires_at"),
        step_up_verified_at: row.get("step_up_verified_at"),
        step_up_expires_at: row.get("step_up_expires_at"),
        revoked_at: row.get("revoked_at"),
        ip: row.get("ip"),
        user_agent: row.get("user_agent"),
        accept_language: None,
        accept: None,
        accept_encoding: None,
        sec_fetch_site: None,
        sec_fetch_mode: None,
        sec_fetch_dest: None,
        sec_ch_ua: None,
        sec_ch_ua_arch: None,
        sec_ch_ua_bitness: None,
        sec_ch_ua_full_version: None,
        sec_ch_ua_full_version_list: None,
        sec_ch_ua_model: None,
        sec_ch_ua_wow64: None,
        sec_ch_ua_form_factors: None,
        sec_ch_ua_platform: None,
        sec_ch_ua_platform_version: None,
        sec_ch_ua_mobile: None,
        cookie_theft_risk_score: row.get("cookie_theft_risk_score"),
        cookie_theft_detected_at: row.get("cookie_theft_detected_at"),
        account_device_id: account_device_id_val.map(|u| u.to_string()),
        device_trust_level: row.get("device_trust_level"),
        device_trust_score: row.get("device_trust_score"),
        risk_score: row.get("risk_score"),
        risk_decision: row.get("risk_decision"),
        activity_window_started_at: row.get("activity_window_started_at"),
        activity_request_count: req_count as u32,
        last_activity_risk_event_at: row.get("last_activity_risk_event_at"),
    }))
}

pub async fn revoke_session_db(db: &PgPool, session_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE user_sessions SET revoked_at = NOW() WHERE session_id = $1")
        .bind(session_id)
        .execute(db)
        .await?;

    Ok(())
}
