use axum::{
    Router,
    routing::{delete, get, post},
};

use crate::app::AppState;

#[path = "identity.domains.service_accounts.routes.manage.rs"]
pub(crate) mod manage;
#[path = "identity.domains.service_accounts.routes.oauth_clients.rs"]
pub(crate) mod oauth_clients;
#[path = "identity.domains.service_accounts.routes.state.rs"]
pub(crate) mod state;

#[cfg(test)]
#[path = "identity.domains.service_accounts.routes.tests.rs"]
mod tests;

pub use manage::{
    create_service_account, get_service_account, list_service_accounts, update_service_account,
};
pub(crate) use oauth_clients::{
    attach_oauth_client, create_oauth_client, revoke_oauth_client, rotate_oauth_client_secret,
};
pub use state::{reactivate_service_account, suspend_service_account};

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspaceId}/service-accounts",
            get(list_service_accounts).post(create_service_account),
        )
        .route(
            "/workspaces/{workspaceId}/service-accounts/{serviceAccountId}",
            get(get_service_account).patch(update_service_account),
        )
        .route(
            "/workspaces/{workspaceId}/service-accounts/{serviceAccountId}/suspend",
            post(suspend_service_account),
        )
        .route(
            "/workspaces/{workspaceId}/service-accounts/{serviceAccountId}/reactivate",
            post(reactivate_service_account),
        )
        .route(
            "/workspaces/{workspaceId}/service-accounts/{serviceAccountId}/oauth-clients",
            post(create_oauth_client),
        )
        .route(
            "/workspaces/{workspaceId}/service-accounts/{serviceAccountId}/oauth-clients:attach",
            post(attach_oauth_client),
        )
        .route(
            "/workspaces/{workspaceId}/service-accounts/{serviceAccountId}/oauth-clients/{clientId}/rotate-secret",
            post(rotate_oauth_client_secret),
        )
        .route(
            "/workspaces/{workspaceId}/service-accounts/{serviceAccountId}/oauth-clients/{clientId}",
            delete(revoke_oauth_client),
        )
}
