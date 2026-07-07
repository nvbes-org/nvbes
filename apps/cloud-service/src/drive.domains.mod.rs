#[path = "drive.domains.audit.mod.rs"]
pub mod audit;
#[path = "drive.domains.auth.mod.rs"]
pub mod auth;
#[path = "drive.domains.authz.mod.rs"]
pub mod authz;
#[path = "drive.domains.billing.mod.rs"]
pub mod billing;
#[path = "drive.domains.files.mod.rs"]
pub mod files;
#[path = "drive.domains.members.mod.rs"]
pub mod members;
#[path = "drive.domains.privacy.mod.rs"]
pub mod privacy;
#[path = "drive.domains.public_api.mod.rs"]
pub mod public_api;
#[path = "drive.domains.quotas.mod.rs"]
pub mod quotas;
#[path = "drive.domains.share_links.mod.rs"]
pub mod share_links;
#[path = "drive.domains.uploads.mod.rs"]
pub mod uploads;
#[path = "drive.domains.workspaces.mod.rs"]
pub mod workspaces;

use axum::Router;

pub fn router(state: &crate::app::AppState) -> Router<crate::app::AppState> {
    Router::new()
        .merge(auth::router(state))
        .merge(workspaces::router(state))
        .merge(members::router(state))
        .merge(files::router(state))
        .merge(uploads::router(state))
        .merge(share_links::router(state))
        .merge(quotas::router(state))
        .merge(billing::router(state))
        .merge(audit::router(state))
        .merge(privacy::router(state))
        .merge(public_api::router(state))
}
