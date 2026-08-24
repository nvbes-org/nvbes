#[path = "trust_risk.app.rs"]
mod app;
#[path = "trust_risk.assessment.db.rs"]
mod assessment_db;
#[path = "trust_risk.assessment.grpc.rs"]
mod assessment_grpc;
#[path = "trust_risk.auth.rs"]
mod auth;
#[path = "trust_risk.config.rs"]
mod config;
#[path = "trust_risk.database.rs"]
mod database;
#[path = "trust_risk.health.rs"]
mod health;
#[path = "trust_risk.ingress.db.rs"]
mod ingress_db;
#[path = "trust_risk.ingress.grpc.rs"]
mod ingress_grpc;
#[path = "trust_risk.projection.rs"]
mod projection;
#[path = "trust_risk.projection.db.rs"]
mod projection_db;
#[path = "trust_risk.metrics.rs"]
mod risk_metrics;

fn main() {}
