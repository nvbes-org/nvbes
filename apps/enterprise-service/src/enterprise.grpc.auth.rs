use tonic::{Request, Status};

const ENTERPRISE_GRPC_AUTH_TOKEN_ENV: &str = "NVBES_ENTERPRISE_GRPC_AUTH_TOKEN";
const MIN_AUTH_TOKEN_LENGTH: usize = 32;

#[derive(Clone)]
pub struct EnterpriseGrpcAuthenticator {
    expected_token: Option<String>,
}

impl EnterpriseGrpcAuthenticator {
    pub fn from_env(environment: &str) -> Result<Self, String> {
        Self::from_value(
            environment,
            std::env::var(ENTERPRISE_GRPC_AUTH_TOKEN_ENV).ok(),
        )
    }

    fn from_value(environment: &str, token: Option<String>) -> Result<Self, String> {
        let token = token
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        if environment != "development" && token.is_none() {
            return Err(format!(
                "{ENTERPRISE_GRPC_AUTH_TOKEN_ENV} must be configured outside development"
            ));
        }
        if token
            .as_ref()
            .is_some_and(|value| value.len() < MIN_AUTH_TOKEN_LENGTH)
        {
            return Err(format!(
                "{ENTERPRISE_GRPC_AUTH_TOKEN_ENV} must contain at least {MIN_AUTH_TOKEN_LENGTH} characters"
            ));
        }

        Ok(Self {
            expected_token: token,
        })
    }

    pub fn authenticate(&self, request: Request<()>) -> Result<Request<()>, Status> {
        let Some(expected_token) = self.expected_token.as_deref() else {
            return Ok(request);
        };
        let provided_token = request
            .metadata()
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .filter(|value| !value.is_empty())
            .ok_or_else(|| Status::unauthenticated("Missing internal gRPC credential."))?;

        if !constant_time_eq(provided_token.as_bytes(), expected_token.as_bytes()) {
            return Err(Status::unauthenticated("Invalid internal gRPC credential."));
        }
        Ok(request)
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

#[cfg(test)]
mod tests {
    use super::EnterpriseGrpcAuthenticator;
    use tonic::Request;

    const VALID_TOKEN: &str = "account-worker-enterprise-token-32";

    #[test]
    fn production_requires_an_internal_credential() {
        let error = EnterpriseGrpcAuthenticator::from_value("production", None)
            .err()
            .expect("missing production token should fail");

        assert!(error.contains("must be configured"));
    }

    #[test]
    fn rejects_requests_without_the_internal_credential() {
        let authenticator =
            EnterpriseGrpcAuthenticator::from_value("production", Some(VALID_TOKEN.to_string()))
                .expect("authenticator should build");

        let error = authenticator
            .authenticate(Request::new(()))
            .expect_err("missing credential should fail");
        assert_eq!(error.code(), tonic::Code::Unauthenticated);
    }

    #[test]
    fn accepts_the_expected_internal_credential() {
        let authenticator =
            EnterpriseGrpcAuthenticator::from_value("production", Some(VALID_TOKEN.to_string()))
                .expect("authenticator should build");
        let mut request = Request::new(());
        request.metadata_mut().insert(
            "authorization",
            format!("Bearer {VALID_TOKEN}")
                .parse()
                .expect("metadata should parse"),
        );

        assert!(authenticator.authenticate(request).is_ok());
    }
}
