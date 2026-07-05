use tonic::transport::Channel;

use crate::pb::nvbes::billing::v1::billing_service_client::BillingServiceClient;

pub async fn billing_client(
    endpoint: &str,
) -> Result<BillingServiceClient<Channel>, tonic::transport::Error> {
    BillingServiceClient::connect(endpoint.to_string()).await
}
