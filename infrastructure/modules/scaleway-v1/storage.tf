resource "scaleway_object_bucket" "files" {
  name          = var.bucket_name
  project_id    = var.project_id
  region        = var.region
  force_destroy = false
  tags          = local.bucket_tags

  versioning {
    enabled = true
  }

  cors_rule {
    allowed_headers = ["authorization", "content-type", "x-amz-date", "x-amz-content-sha256"]
    allowed_methods = ["GET", "PUT", "POST", "HEAD"]
    allowed_origins = var.bucket_cors_allowed_origins
    expose_headers  = ["etag"]
    max_age_seconds = 300
  }

  lifecycle_rule {
    id                                     = "abort-incomplete-multipart"
    enabled                                = true
    abort_incomplete_multipart_upload_days = 1
  }

  lifecycle_rule {
    id      = "expire-pending-uploads"
    prefix  = "pending/"
    enabled = true

    expiration {
      days = 7
    }
  }

  lifecycle_rule {
    id      = "expire-trash"
    prefix  = "trash/"
    enabled = true

    expiration {
      days = 180
    }
  }
}
