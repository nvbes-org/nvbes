output "worker_endpoint" {
  description = "Public URL of the Worker serverless container."
  value       = scaleway_container.worker.public_endpoint
}

output "worker_registry_endpoint" {
  description = "Scaleway Container Registry endpoint for Worker images."
  value       = scaleway_registry_namespace.worker.endpoint
}
