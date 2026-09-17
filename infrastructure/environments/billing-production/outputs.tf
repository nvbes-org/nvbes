output "billing_endpoint" {
  value       = scaleway_container.billing.public_endpoint
  description = "Public HTTPS endpoint for the isolated Billing runtime."
}

output "billing_container_id" {
  value       = scaleway_container.billing.id
  description = "Scaleway serverless container identifier for Billing."
}

output "billing_database_id" {
  value       = scaleway_sdb_sql_database.billing.id
  description = "Scaleway serverless SQL database identifier for Billing."
}

output "billing_database_name" {
  value       = scaleway_sdb_sql_database.billing.name
  description = "Database name for Billing."
}

output "billing_migration_job_id" {
  value       = scaleway_job_definition.billing_database_migration.id
  description = "Scaleway Job definition ID for Billing schema migrations."
}

output "billing_registry_endpoint" {
  value       = scaleway_registry_namespace.billing.endpoint
  description = "Scaleway container registry endpoint for Billing images."
}
