output "audit_archive_bucket_name" {
  value = module.audit_archive.bucket_name
}

output "writer_access_key" {
  value     = module.audit_archive.writer_access_key
  sensitive = true
}

output "writer_secret_key" {
  description = "Move immediately to the production secret manager; never copy into tfvars."
  value       = module.audit_archive.writer_secret_key
  sensitive   = true
}
