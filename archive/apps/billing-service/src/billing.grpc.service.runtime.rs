use std::{future::Future, net::SocketAddr};

use tonic::transport::Server;

use crate::{
    app::BillingAppState,
    grpc::pb::nvbes::billing::v1::billing_service_server::BillingServiceServer,
};

#[derive(Clone)]
pub(super) struct BillingGrpcService {
    pub(super) state: BillingAppState,
}

impl BillingGrpcService {
    fn new(state: BillingAppState) -> Self {
        Self { state }
    }

    fn into_server(self) -> BillingServiceServer<Self> {
        BillingServiceServer::new(self)
    }
}

pub async fn serve(
    addr: SocketAddr,
    state: BillingAppState,
    shutdown: impl Future<Output = ()>,
) -> Result<(), tonic::transport::Error> {
    Server::builder()
        .add_service(BillingGrpcService::new(state).into_server())
        .serve_with_shutdown(addr, shutdown)
        .await
}
