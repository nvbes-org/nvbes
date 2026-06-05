output "api_domain" {
  value = local.api_domain
}

output "web_domain" {
  value = local.web_domain
}

output "private_network_id" {
  value = module.scaleway.private_network_id
}

output "object_bucket_name" {
  value = module.scaleway.object_bucket_name
}

output "secret_inventory" {
  description = "Secrets to create/populate in Scaleway Secret Manager outside Terraform state."
  value       = local.secret_inventory
}
