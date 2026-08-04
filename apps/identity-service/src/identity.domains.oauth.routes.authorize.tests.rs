use super::AuthorizeRequest;
use axum::{extract::Query, http::Uri};

#[test]
fn accepts_rfc_9126_authorization_request_with_par_reference() {
    let uri: Uri = "/oauth/authorize?client_id=account-web&request_uri=urn%3Aietf%3Aparams%3Aoauth%3Arequest_uri%3Agxpar_test"
        .parse()
        .expect("valid authorization URI");

    let Query(request) =
        Query::<AuthorizeRequest>::try_from_uri(&uri).expect("valid PAR authorization request");

    assert_eq!(request.client_id, "account-web");
    assert_eq!(
        request.request_uri.as_deref(),
        Some("urn:ietf:params:oauth:request_uri:gxpar_test")
    );
}
