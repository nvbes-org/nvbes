output "bucket_name" {
  value = scaleway_object_bucket.terraform_state.name
}

output "state_keys" {
  value = {
    for stack, path in local.state_paths : stack => path.state
  }
}

output "application_ids" {
  value = {
    for stack, application in scaleway_iam_application.state : stack => application.id
  }
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
