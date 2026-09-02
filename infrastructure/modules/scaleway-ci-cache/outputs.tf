output "project_id" {
  description = "Dedicated Scaleway cache project ID."
  value       = scaleway_account_project.ci_cache.id
}

output "registry_endpoint" {
  description = "Private OCI endpoint used by the BuildKit registry exporter."
  value       = scaleway_registry_namespace.ci_cache.endpoint
}

output "bucket_name" {
  description = "S3-compatible bucket available to compiler and package caches."
  value       = scaleway_object_bucket.ci_cache.name
}

output "active_credential_slot" {
  description = "Credential slot currently synchronized to GitHub."
  value       = local.active_credential_slot
}

output "next_rotation_at" {
  description = "Next boundary at which a Terraform apply rotates the inactive slot."
  value = timecmp(
    time_rotating.ci_cache["a"].rotation_rfc3339,
    time_rotating.ci_cache["b"].rotation_rfc3339,
  ) < 0 ? time_rotating.ci_cache["a"].rotation_rfc3339 : time_rotating.ci_cache["b"].rotation_rfc3339
}
