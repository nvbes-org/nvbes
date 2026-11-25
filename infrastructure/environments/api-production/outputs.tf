output "api_endpoint" {
  description = "Public URL of the API serverless container."
  value       = scaleway_container.api.public_endpoint
}

output "api_registry_endpoint" {
  description = "Scaleway Container Registry endpoint for API images."
  value       = scaleway_registry_namespace.api.endpoint
}

output "api_database_runtime_url" {
  description = "Data-only database URL for API runtime."
  value       = local.api_database_runtime_url
  sensitive   = true
}

output "api_database_migration_url" {
  description = "DDL-capable database URL for API migrations."
  value       = local.api_database_migration_url
  sensitive   = true
}
