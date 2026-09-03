output "platform_operations_container_id" {
  value       = scaleway_container.platform_operations.id
  description = "Scaleway serverless container ID for platform operations."
}

output "platform_operations_endpoint" {
  value       = scaleway_container.platform_operations.public_endpoint
  description = "Internal endpoint for the operator cockpit."
}
