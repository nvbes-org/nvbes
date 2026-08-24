#[path = "trust_risk.app.rs"]
mod app;
#[path = "trust_risk.assessment.db.rs"]
mod assessment_db;
#[path = "trust_risk.assessment.grpc.rs"]
mod assessment_grpc;
#[path = "trust_risk.audit.rs"]
mod audit;
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
#[path = "trust_risk.labels.db.rs"]
mod labels_db;
#[path = "trust_risk.labels.grpc.rs"]
mod labels_grpc;
#[path = "trust_risk.operations.grpc.rs"]
mod operations_grpc;
#[path = "trust_risk.operations.types.rs"]
mod operations_types;
#[path = "trust_risk.projection.rs"]
mod projection;
#[path = "trust_risk.projection.db.rs"]
mod projection_db;
#[path = "trust_risk.retention.rs"]
mod retention;
#[path = "trust_risk.review.db.rs"]
mod review_db;
#[path = "trust_risk.metrics.rs"]
mod risk_metrics;
#[path = "trust_risk.rules.db.rs"]
mod rules_db;

fn main() {}
