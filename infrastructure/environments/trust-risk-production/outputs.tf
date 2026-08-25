output "trust_risk_endpoint" {
  description = "Public HTTP/2 endpoint for authenticated Trust/Risk gRPC APIs."
  value       = scaleway_container.trust_risk.public_endpoint
}

output "trust_risk_registry_endpoint" {
  description = "Private Scaleway Registry receiving verified Trust/Risk images."
  value       = scaleway_registry_namespace.trust_risk.endpoint
}

output "trust_risk_database_id" {
  description = "Dedicated Serverless SQL database used by Trust/Risk."
  value       = scaleway_sdb_sql_database.trust_risk.id
}

output "trust_risk_database_endpoint" {
  description = "Non-secret endpoint of the Trust/Risk database."
  value       = scaleway_sdb_sql_database.trust_risk.endpoint
}

output "trust_risk_database_runtime_url" {
  description = "Sensitive data-only database URL consumed by runtime validation."
  sensitive   = true
  value       = local.trust_risk_database_runtime_url
}

output "trust_risk_database_migration_job_id" {
  description = "Job definition that applies Trust/Risk SQLx migrations."
  value       = scaleway_job_definition.trust_risk_database_migration.id
}

output "trust_risk_grafana_folder_uid" {
  description = "Grafana folder containing the production Trust/Risk dashboard and alerts."
  value       = grafana_folder.trust_risk.uid
}

output "trust_risk_grafana_dashboard_uid" {
  description = "Stable UID of the production Trust/Risk Grafana dashboard."
  value       = grafana_dashboard.trust_risk.uid
}
