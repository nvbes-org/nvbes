use axum::http::HeaderMap;
use nvbes_observability::request_id_from_headers;

pub struct PublicApiRequestMeta {
    request_id: String,
    ip: Option<String>,
    user_agent: Option<String>,
}

impl PublicApiRequestMeta {
    pub fn from_headers(headers: &HeaderMap) -> Self {
        Self {
            request_id: request_id_from_headers(headers),
            ip: crate::http::request::client_ip(headers),
            user_agent: crate::http::request::user_agent(headers),
        }
    }

    pub fn request_id_owned(&self) -> String {
        self.request_id.clone()
    }

    pub fn ip(&self) -> Option<&str> {
        self.ip.as_deref()
    }

    pub fn ip_owned(&self) -> Option<String> {
        self.ip.clone()
    }

    pub fn user_agent(&self) -> Option<&str> {
        self.user_agent.as_deref()
    }

    pub fn user_agent_owned(&self) -> Option<String> {
        self.user_agent.clone()
    }
}
