output "terraform_state_bucket" {
  value = module.terraform_state.bucket_name
}

output "terraform_state_keys" {
  value = module.terraform_state.state_keys
}

output "state_application_ids" {
  value = module.terraform_state.application_ids
}

output "state_access_keys" {
  value     = module.terraform_state.access_keys
  sensitive = true
}

output "state_secret_keys" {
  value     = module.terraform_state.secret_keys
  sensitive = true
}

output "ci_cache_project_id" {
  value = module.ci_cache.project_id
}

output "ci_cache_registry_endpoint" {
  value = module.ci_cache.registry_endpoint
}

output "ci_cache_bucket_name" {
  value = module.ci_cache.bucket_name
}

output "ci_cache_active_credential_slot" {
  value = module.ci_cache.active_credential_slot
}

output "ci_cache_next_rotation_at" {
  value = module.ci_cache.next_rotation_at
}
