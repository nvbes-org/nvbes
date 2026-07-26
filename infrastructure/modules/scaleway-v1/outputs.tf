output "private_network_id" {
  description = "Private network ID."
  value       = scaleway_vpc_private_network.main.id
}

output "api_public_ip" {
  description = "API instance public IPv4 address."
  value       = scaleway_instance_ip.api.address
}

output "worker_public_ip" {
  description = "Legacy Worker instance public IPv4 address (if enabled)."
  value       = try(scaleway_instance_ip.worker[0].address, null)
}

output "postgres_endpoint" {
  description = "Private PostgreSQL endpoint metadata."
  value       = scaleway_rdb_instance.postgres.private_network
  sensitive   = true
}

output "object_bucket_name" {
  description = "Private object bucket name."
  value       = scaleway_object_bucket.files.name
}

output "object_bucket_endpoint" {
  description = "Private object bucket endpoint."
  value       = scaleway_object_bucket.files.endpoint
}

output "runtime_access_key" {
  description = "Runtime IAM access key. Store it in Secret Manager and avoid exposing it in logs."
  value       = scaleway_iam_api_key.runtime.access_key
  sensitive   = true
}

output "runtime_secret_key" {
  description = "Runtime IAM secret key. This is shown once by Scaleway and stored in Terraform state."
  value       = scaleway_iam_api_key.runtime.secret_key
  sensitive   = true
}
