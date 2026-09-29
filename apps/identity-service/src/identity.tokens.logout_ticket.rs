use super::TokenService;
use crate::{
    oauth::{clients::ClientRegistry, logout_request::LogoutRequest},
    tokens_error::TokenError,
};
use chrono::Utc;
use jsonwebtoken::{Algorithm, Validation, decode, decode_header};
use serde::{Deserialize, Serialize};

const AUDIENCE: &str = "nvbes-identity-logout-confirmation";
const TYPE: &str = "nvbes-logout+jwt";

#[derive(Serialize, Deserialize)]
struct Ticket {
    iss: String,
    aud: String,
    iat: u64,
    exp: u64,
    request: LogoutRequest,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::tests::config;

    #[test]
    fn ticket_expiry_and_type_are_distinct_from_authentication_tokens() {
        let clients = ClientRegistry::from_json(r#"[{"client_id":"account-web","display_name":"Account","redirect_uris":["https://account.example/callback"],"post_logout_redirect_uris":[],"resources":{"https://api.example/account":{"audience":"nvbes-account-service","scopes":["account:read"]}},"allow_refresh":false,"require_dpop":false}]"#, false).unwrap();
        let service = TokenService::new(config()).unwrap();
        let request = || LogoutRequest::from_fields(vec![], &clients, &service).unwrap();
        let ticket = service.logout_ticket(request()).unwrap();
        assert!(service.read_logout_ticket(&ticket, &clients).is_ok());
        assert!(service.verify_logout_hint(&ticket).is_err());
        assert!(service.verify(&ticket, "nvbes-account-service").is_err());
        let now = Utc::now().timestamp() as u64;
        for (iat, exp, typ) in [
            (now - 301, now - 1, TYPE),
            (now, now + 301, TYPE),
            (now + 1, now + 100, TYPE),
            (now, now + 100, "JWT"),
            (now, now + 100, "at+jwt"),
        ] {
            let ticket = service
                .keys
                .sign(
                    typ,
                    &Ticket {
                        iss: service.issuer.clone(),
                        aud: AUDIENCE.into(),
                        iat,
                        exp,
                        request: request(),
                    },
                )
                .unwrap();
            assert!(service.read_logout_ticket(&ticket, &clients).is_err());
        }
    }
}

impl TokenService {
    /// Short-lived, integrity-protected navigation context. Contains no ID Token
    /// or cookie secrets and grants no right to revoke without session CSRF.
    pub fn logout_ticket(&self, request: LogoutRequest) -> Result<String, TokenError> {
        let now = Utc::now().timestamp() as u64;
        self.keys.sign(
            TYPE,
            &Ticket {
                iss: self.issuer.clone(),
                aud: AUDIENCE.into(),
                iat: now,
                exp: now + 300,
                request,
            },
        )
    }

    pub fn read_logout_ticket(
        &self,
        value: &str,
        clients: &ClientRegistry,
    ) -> Result<LogoutRequest, TokenError> {
        if value.len() > 16_384 {
            return Err(TokenError::InvalidToken);
        }
        let now = Utc::now().timestamp() as u64;
        let header = decode_header(value)?;
        if header.alg != Algorithm::RS256
            || header.typ.as_deref() != Some(TYPE)
            || header.jku.is_some()
            || header.jwk.is_some()
            || header.x5u.is_some()
        {
            return Err(TokenError::InvalidToken);
        }
        let key = self
            .keys
            .decoding_key(header.kid.as_deref().ok_or(TokenError::InvalidToken)?, now)?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[AUDIENCE]);
        validation.leeway = 0;
        let ticket = decode::<Ticket>(value, key, &validation)?.claims;
        if ticket.iat > now
            || ticket.exp <= now
            || ticket.exp <= ticket.iat
            || ticket.exp - ticket.iat > 300
        {
            return Err(TokenError::InvalidToken);
        }
        ticket
            .request
            .revalidate(clients)
            .map_err(|_| TokenError::InvalidToken)?;
        Ok(ticket.request)
    }
}
