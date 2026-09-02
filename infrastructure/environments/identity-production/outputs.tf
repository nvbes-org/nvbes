output "identity_endpoint" {
  description = "Public endpoint exposing only Identity health and protected metrics."
  value       = scaleway_container.identity.public_endpoint
}

output "identity_registry_endpoint" {
  description = "Private Scaleway Registry receiving verified Identity images."
  value       = scaleway_registry_namespace.identity.endpoint
}

output "identity_database_id" {
  description = "Dedicated Serverless SQL database used by Identity."
  value       = scaleway_sdb_sql_database.identity.id
}

output "identity_database_runtime_url" {
  description = "Sensitive data-only database URL consumed by runtime validation."
  sensitive   = true
  value       = local.identity_database_runtime_url
}

output "identity_database_migration_job_id" {
  description = "Job definition applying Identity SQLx migrations."
  value       = scaleway_job_definition.identity_database_migration.id
}

output "identity_synthetic_auth_job_id" {
  description = "Job definition proving Identity authentication and recovery without public routes."
  value       = scaleway_job_definition.identity_synthetic_auth.id
}
