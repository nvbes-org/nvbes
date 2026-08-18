output "api_domain" {
  value = local.api_domain
}

output "private_network_id" {
  value = module.scaleway.private_network_id
}

output "audit_archive_bucket_name" {
  value = module.scaleway.audit_archive_bucket_name
}

output "audit_anchor_signer_access_key" {
  value     = module.scaleway.audit_anchor_signer_access_key
  sensitive = true
}

output "audit_anchor_signer_secret_key" {
  value     = module.scaleway.audit_anchor_signer_secret_key
  sensitive = true
}
