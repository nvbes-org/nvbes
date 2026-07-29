use aws_sdk_s3::Client as S3Client;

pub(super) fn new_client(
    endpoint: &str,
    region: &str,
    access_key: &str,
    secret_key: &str,
) -> S3Client {
    let credentials =
        aws_sdk_s3::config::Credentials::new(access_key, secret_key, None, None, "nvbes");

    let config = aws_sdk_s3::Config::builder()
        .region(aws_sdk_s3::config::Region::new(region.to_string()))
        .endpoint_url(endpoint)
        .credentials_provider(credentials)
        .behavior_version_latest()
        .force_path_style(true)
        .build();

    S3Client::from_conf(config)
}
