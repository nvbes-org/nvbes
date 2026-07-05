use chrono::Utc;
use uuid::Uuid;

use crate::domains::{
    auth::jwt::JwtService,
    auth::password::token_hash,
    auth::sessions::cache::{cached_session_from_login, current_session_ttl},
    service_accounts::routes::tests::seed::ServiceAccountRouteFixture,
};

pub(super) async fn seed_admin_session(
    redis: &nvbes_redis::RedisPool,
    jwt: &JwtService,
    fixture: &mut ServiceAccountRouteFixture,
    now: chrono::DateTime<Utc>,
) {
    let token_pair = jwt
        .generate_token_pair_with_session(
            fixture.admin_principal_id,
            Some(fixture.workspace_id),
            None,
            "drive.workspace.manage drive.files.read",
            Some(Uuid::new_v4()),
            Some(fixture.tenant_id),
            None,
            Some("aal2"),
            Some(vec!["pwd".to_string(), "otp".to_string()]),
            Some("identity-admin-client"),
            Some(now.timestamp()),
            None,
        )
        .expect("token pair should be created");

    let mut cached_session = cached_session_from_login(
        token_pair.session_id,
        fixture.admin_principal_id,
        Some(fixture.tenant_id),
        None,
        Some(fixture.workspace_id),
        None,
        None,
        token_hash(&token_pair.access_token),
        Some("aal2".to_string()),
        vec!["pwd".to_string(), "otp".to_string()],
        now,
        now,
        None,
        None,
        now + chrono::Duration::hours(2),
    );
    cached_session.step_up_verified_at = Some(now);
    cached_session.step_up_expires_at = Some(now + chrono::Duration::minutes(30));
    nvbes_redis::session::set_session(redis, &cached_session, current_session_ttl(&cached_session))
        .await
        .expect("redis session insert should succeed");

    fixture.admin_token = token_pair.access_token;
}
