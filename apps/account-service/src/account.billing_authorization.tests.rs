use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use tower::ServiceExt;

#[tokio::test]
async fn only_active_personal_owners_and_consistent_team_owners_are_allowed() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
    let db = crate::database::connect(&url, 2).await.unwrap();
    crate::database::migrate(&db).await.unwrap();
    let owner = Uuid::new_v4();
    let member = Uuid::new_v4();
    let (team, _) = crate::teams::create_and_join_for_synthetic(&db, owner, member)
        .await
        .unwrap();
    let target = |principal_id, account_id, account_type| Target {
        principal_id,
        account_id,
        account_type,
    };
    assert!(
        allowed(&db, &target(owner, owner, AccountType::Principal))
            .await
            .unwrap()
    );
    assert!(
        !allowed(&db, &target(member, owner, AccountType::Principal))
            .await
            .unwrap()
    );
    assert!(
        !allowed(&db, &target(owner, team, AccountType::Principal))
            .await
            .unwrap()
    );
    assert!(
        allowed(&db, &target(owner, team, AccountType::Team))
            .await
            .unwrap()
    );
    assert!(
        !allowed(&db, &target(member, team, AccountType::Team))
            .await
            .unwrap()
    );
    assert!(
        !allowed(&db, &target(Uuid::new_v4(), team, AccountType::Team))
            .await
            .unwrap()
    );
    sqlx::query(
        "UPDATE account_team_memberships SET role='member' WHERE team_id=$1 AND principal_id=$2",
    )
    .bind(team)
    .bind(owner)
    .execute(&db)
    .await
    .unwrap();
    assert!(
        !allowed(&db, &target(owner, team, AccountType::Team))
            .await
            .unwrap()
    );
    sqlx::query(
        "UPDATE account_team_memberships SET role='owner' WHERE team_id=$1 AND principal_id=$2",
    )
    .bind(team)
    .bind(owner)
    .execute(&db)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE account_profiles SET lifecycle_status='closure_pending' WHERE principal_id=$1",
    )
    .bind(owner)
    .execute(&db)
    .await
    .unwrap();
    assert!(
        !allowed(&db, &target(owner, owner, AccountType::Principal))
            .await
            .unwrap()
    );
    assert!(
        !allowed(&db, &target(owner, team, AccountType::Team))
            .await
            .unwrap()
    );
    sqlx::query("UPDATE account_profiles SET lifecycle_status='active' WHERE principal_id=$1")
        .bind(owner)
        .execute(&db)
        .await
        .unwrap();
    sqlx::query("UPDATE account_teams SET status='closed' WHERE id=$1")
        .bind(team)
        .execute(&db)
        .await
        .unwrap();
    assert!(
        !allowed(&db, &target(owner, team, AccountType::Team))
            .await
            .unwrap()
    );

    let secret = "a".repeat(64);
    let app = router(db.clone(), Some(&secret));
    let body = serde_json::to_vec(&target(owner, owner, AccountType::Principal)).unwrap();
    let request = |credentials: &[&str], body: Vec<u8>| {
        let mut request = Request::post("/internal/v1/billing/authorize")
            .header("content-type", "application/json");
        for credential in credentials {
            request = request.header("authorization", *credential);
        }
        request.body(Body::from(body)).unwrap()
    };
    let credential = format!("Bearer {secret}");
    for headers in [
        vec![],
        vec!["Bearer invalid"],
        vec![credential.as_str(), credential.as_str()],
    ] {
        let response = app
            .clone()
            .oneshot(request(&headers, body.clone()))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(response.headers()["cache-control"], "no-store");
    }
    let response = app
        .clone()
        .oneshot(request(&[&credential], body.clone()))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let response: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1024).await.unwrap()).unwrap();
    assert_eq!(response["allowed"], true);
    assert_eq!(response["principal_id"], owner.to_string());
    assert_eq!(
        app.clone()
            .oneshot(request(&[&credential], vec![b' '; 1025]))
            .await
            .unwrap()
            .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(
        router(db.clone(), None)
            .oneshot(request(&[&credential], body.clone()))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    db.close().await;
    assert_eq!(
        app.oneshot(request(&[&credential], body))
            .await
            .unwrap()
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
}
