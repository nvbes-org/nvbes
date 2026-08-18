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
