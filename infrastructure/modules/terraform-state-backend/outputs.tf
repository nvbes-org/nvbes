output "bucket_name" {
  value = scaleway_object_bucket.terraform_state.name
}

output "state_keys" {
  value = {
    for stack, path in local.state_paths : stack => path.state
  }
}

output "application_ids" {
  value = local.state_application_ids
}

output "access_keys" {
  value = {
    for stack, key in scaleway_iam_api_key.state : stack => key.access_key
  }
  sensitive = true
}

output "secret_keys" {
  value = {
    for stack, key in scaleway_iam_api_key.state : stack => key.secret_key
  }
  sensitive = true
}
