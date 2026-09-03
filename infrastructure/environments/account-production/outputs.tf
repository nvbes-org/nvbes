output "account_endpoint" {
  description = "Public endpoint exposing only Account health and protected metrics."
  value       = scaleway_container.account.public_endpoint
}

output "account_registry_endpoint" {
  description = "Private Scaleway Registry receiving verified Account images."
  value       = scaleway_registry_namespace.account.endpoint
}

output "account_database_id" {
  description = "Dedicated Serverless SQL database used by Account."
  value       = scaleway_sdb_sql_database.account.id
}

output "account_database_runtime_url" {
  description = "Sensitive data-only database URL consumed by runtime validation."
  sensitive   = true
  value       = local.account_database_runtime_url
}

output "account_database_migration_job_id" {
  description = "Job definition applying Account SQLx migrations."
  value       = scaleway_job_definition.account_database_migration.id
}

output "account_synthetic_smoke_job_id" {
  description = "Job definition proving Account lifecycle and outbox without public exposure."
  value       = scaleway_job_definition.account_synthetic_smoke.id
}

output "account_privacy_jobs_job_id" {
  description = "Job definition processing Account privacy background jobs."
  value       = scaleway_job_definition.account_privacy_jobs.id
}
