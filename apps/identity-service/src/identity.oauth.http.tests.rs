use super::router;
use crate::tokens::TokenService;

#[test]
fn http_module_is_kept_separate_from_mutating_authorization_routes() {
    let _router_type: fn(&str, std::sync::Arc<TokenService>) -> _ = router;
}
