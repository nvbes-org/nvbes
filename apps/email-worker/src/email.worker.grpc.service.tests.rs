use nvbes_email::proto::nvbes::email::v1::{
    SubmitEmailRequest, email_delivery_service_server::EmailDeliveryService,
};
use tonic::{Code, Request};

use super::{EmailDeliveryGrpcService, map_accept_error};
use crate::{database::AcceptCommandError, test_support};

#[sqlx::test(migrations = "./migrations")]
async fn submit_email_enforces_authentication_validation_and_idempotency(pool: sqlx::PgPool) {
    let service = EmailDeliveryGrpcService::new(test_support::state(pool));
    let command = test_support::command("grpc-submit", "grpc@example.com");

    let error = service
        .submit_email(Request::new(command.clone().into_proto()))
        .await
        .unwrap_err();
    assert_eq!(error.code(), Code::Unauthenticated);

    let mut invalid = Request::new(SubmitEmailRequest {
        producer: "identity-service".to_string(),
        ..Default::default()
    });
    test_support::authorize(&mut invalid);
    assert_eq!(
        service.submit_email(invalid).await.unwrap_err().code(),
        Code::InvalidArgument
    );

    let mut first = Request::new(command.clone().into_proto());
    test_support::authorize(&mut first);
    let first = service.submit_email(first).await.unwrap().into_inner();
    assert!(!first.duplicate);

    let mut duplicate = Request::new(command.clone().into_proto());
    test_support::authorize(&mut duplicate);
    assert!(
        service
            .submit_email(duplicate)
            .await
            .unwrap()
            .into_inner()
            .duplicate
    );

    let mut conflicting = command;
    conflicting.recipient.email = "other@example.com".to_string();
    let mut conflicting = Request::new(conflicting.into_proto());
    test_support::authorize(&mut conflicting);
    assert_eq!(
        service.submit_email(conflicting).await.unwrap_err().code(),
        Code::AlreadyExists
    );
}

#[test]
fn persistence_error_mapping_never_exposes_internal_details() {
    assert_eq!(
        map_accept_error(AcceptCommandError::Invalid).code(),
        Code::InvalidArgument
    );
    assert_eq!(
        map_accept_error(AcceptCommandError::Conflict).code(),
        Code::AlreadyExists
    );
    assert_eq!(
        map_accept_error(AcceptCommandError::Database(sqlx::Error::RowNotFound)).code(),
        Code::Unavailable
    );
    assert_eq!(
        map_accept_error(AcceptCommandError::Encryption(anyhow::anyhow!("secret"))).code(),
        Code::Unavailable
    );
}
