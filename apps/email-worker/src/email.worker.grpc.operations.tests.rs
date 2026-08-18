use nvbes_email::proto::nvbes::email::v1::{
    ApplyEmailSuppressionRequest, GetEmailOperationsSnapshotRequest,
    GetEmailPrivacyActivityRequest, ReleaseEmailSuppressionRequest, ReplayEmailRequest,
    ReviewEmailSuppressionRequest, email_operations_service_server::EmailOperationsService,
};
use tonic::{Code, Request};

use super::{EmailOperationsGrpcService, action_error, internal};
use crate::{crypto::EmailCrypto, database, operations_actions::ActionError, test_support};

fn authorized<T>(payload: T) -> Request<T> {
    let mut request = Request::new(payload);
    test_support::authorize(&mut request);
    request
}

#[sqlx::test(migrations = "./migrations")]
async fn operations_api_covers_authorized_queries_and_operator_lifecycle(pool: sqlx::PgPool) {
    let service = EmailOperationsGrpcService::new(test_support::state(pool.clone()));
    assert_eq!(
        service
            .get_operations_snapshot(Request::new(GetEmailOperationsSnapshotRequest::default()))
            .await
            .unwrap_err()
            .code(),
        Code::InvalidArgument
    );

    let snapshot = service
        .get_operations_snapshot(authorized(GetEmailOperationsSnapshotRequest {
            caller: Some(test_support::caller("backoffice-service")),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(snapshot.queued_message_count, 0);

    let invalid_privacy = GetEmailPrivacyActivityRequest {
        caller: Some(test_support::caller("identity-service")),
        email: "invalid".to_string(),
    };
    assert_eq!(
        service
            .get_privacy_activity(authorized(invalid_privacy))
            .await
            .unwrap_err()
            .code(),
        Code::InvalidArgument
    );
    let privacy = service
        .get_privacy_activity(authorized(GetEmailPrivacyActivityRequest {
            caller: Some(test_support::caller("identity-service")),
            email: "privacy@example.com".to_string(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(privacy.messages.is_empty());

    let email = "operator@example.com";
    let applied = service
        .apply_suppression(authorized(ApplyEmailSuppressionRequest {
            operator: Some(test_support::operator()),
            email: email.to_string(),
            scope: "all".to_string(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(applied.status, "applied");
    let reviewed = service
        .review_suppression(authorized(ReviewEmailSuppressionRequest {
            operator: Some(test_support::operator()),
            email: email.to_string(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(reviewed.status, "reviewed");
    let released = service
        .release_suppression(authorized(ReleaseEmailSuppressionRequest {
            operator: Some(test_support::operator()),
            email: email.to_string(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(released.status, "released");

    assert_eq!(
        service
            .apply_suppression(authorized(ApplyEmailSuppressionRequest {
                operator: Some(test_support::operator()),
                email: email.to_string(),
                scope: "invalid".to_string(),
            }))
            .await
            .unwrap_err()
            .code(),
        Code::InvalidArgument
    );
    assert_eq!(
        service
            .release_suppression(authorized(ReleaseEmailSuppressionRequest {
                operator: Some(test_support::operator()),
                email: "missing@example.com".to_string(),
            }))
            .await
            .unwrap_err()
            .code(),
        Code::NotFound
    );

    assert_eq!(
        service
            .replay_email(authorized(ReplayEmailRequest {
                operator: Some(test_support::operator()),
                message_id: "invalid".to_string(),
            }))
            .await
            .unwrap_err()
            .code(),
        Code::InvalidArgument
    );
    let command = test_support::command("grpc-replay", "replay@example.com");
    let receipt = database::accept_command(
        &pool,
        &EmailCrypto::new([7; 32], [9; 32]),
        "nvbes.fr",
        &command.clone().into_proto(),
        &command,
    )
    .await
    .unwrap();
    let message_id: uuid::Uuid =
        sqlx::query_scalar("SELECT id FROM email_messages WHERE message_id = $1")
            .bind(receipt.receipt.message_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        service
            .replay_email(authorized(ReplayEmailRequest {
                operator: Some(test_support::operator()),
                message_id: message_id.to_string(),
            }))
            .await
            .unwrap_err()
            .code(),
        Code::FailedPrecondition
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn operations_api_rejects_wrong_callers_and_invalid_audit_context(pool: sqlx::PgPool) {
    let service = EmailOperationsGrpcService::new(test_support::state(pool));
    let wrong_caller = authorized(GetEmailOperationsSnapshotRequest {
        caller: Some(test_support::caller("identity-service")),
    });
    assert_eq!(
        service
            .get_operations_snapshot(wrong_caller)
            .await
            .unwrap_err()
            .code(),
        Code::PermissionDenied
    );

    let mut invalid_operator = test_support::operator();
    invalid_operator.actor = "invalid".to_string();
    assert_eq!(
        service
            .apply_suppression(authorized(ApplyEmailSuppressionRequest {
                operator: Some(invalid_operator),
                email: "operator@example.com".to_string(),
                scope: "all".to_string(),
            }))
            .await
            .unwrap_err()
            .code(),
        Code::InvalidArgument
    );
}

#[test]
fn operation_errors_have_stable_grpc_codes() {
    assert_eq!(
        action_error(ActionError::Invalid).code(),
        Code::InvalidArgument
    );
    assert_eq!(action_error(ActionError::NotFound).code(), Code::NotFound);
    assert_eq!(
        action_error(ActionError::Precondition).code(),
        Code::FailedPrecondition
    );
    assert_eq!(
        action_error(ActionError::Database(sqlx::Error::RowNotFound)).code(),
        Code::Unavailable
    );
    assert_eq!(
        action_error(ActionError::Encryption(anyhow::anyhow!("secret"))).code(),
        Code::Unavailable
    );
    assert_eq!(internal("query detail").code(), Code::Unavailable);
}
