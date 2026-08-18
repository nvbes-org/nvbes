use nvbes_email::proto::nvbes::email::v1::{
    SubmitEmailRequest, email_delivery_service_client::EmailDeliveryServiceClient,
};
use tonic::Code;

use super::{
    EmailDeliveryGrpcService, MAX_GRPC_DECODE_BYTES, MAX_PERSISTED_COMMAND_BYTES, delivery_server,
    validate_submission_size,
};

#[test]
fn persistence_budget_rejects_oversized_commands_before_database_access() {
    let request = SubmitEmailRequest {
        producer: "a".repeat(MAX_PERSISTED_COMMAND_BYTES + 1),
        ..Default::default()
    };

    assert_eq!(
        validate_submission_size(&request).unwrap_err().code(),
        Code::InvalidArgument
    );
}

#[tokio::test]
async fn grpc_transport_rejects_messages_above_the_worker_budget() {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let address = listener.local_addr().unwrap();
    let pool = crate::database::connect_lazy("postgres://localhost/email-size-test").unwrap();
    let service = EmailDeliveryGrpcService::new(crate::test_support::state(pool));
    let app = tonic::service::Routes::new(delivery_server(service)).into_axum_router();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let mut client = EmailDeliveryServiceClient::connect(format!("http://{address}"))
        .await
        .unwrap();

    let error = client
        .submit_email(SubmitEmailRequest {
            producer: "a".repeat(MAX_GRPC_DECODE_BYTES + 1),
            ..Default::default()
        })
        .await
        .unwrap_err();

    assert_eq!(error.code(), Code::OutOfRange);
    server.abort();
}
