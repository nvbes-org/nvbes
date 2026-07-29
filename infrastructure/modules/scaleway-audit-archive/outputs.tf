output "bucket_name" {
  value = scaleway_object_bucket.archive.name
}

output "writer_access_key" {
  value     = scaleway_iam_api_key.writer.access_key
  sensitive = true
}

output "writer_secret_key" {
  value     = scaleway_iam_api_key.writer.secret_key
  sensitive = true
}

output "writer_application_id" {
  value = scaleway_iam_application.writer.id
}
