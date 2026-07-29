locals {
  bucket_tags = merge(
    { for tag in var.tags : replace(tag, ":", "_") => tag },
    { data_class = "security_evidence" },
  )
}

resource "scaleway_iam_application" "writer" {
  name        = "${var.name_prefix}-audit-archive-writer"
  description = "Write-only identity for signed audit anchors from the production account."
}

resource "scaleway_iam_policy" "writer" {
  name           = "${var.name_prefix}-audit-archive-writer"
  description    = "Can write archive objects but cannot read, delete, or administer the bucket."
  application_id = scaleway_iam_application.writer.id

  rule {
    project_ids          = [var.project_id]
    permission_set_names = ["ObjectStorageObjectsWrite"]
  }
}

resource "scaleway_iam_api_key" "writer" {
  application_id     = scaleway_iam_application.writer.id
  default_project_id = var.project_id
  description        = "Rotating cross-account credential for signed audit anchor writes."
}

resource "scaleway_object_bucket" "archive" {
  name                = var.bucket_name
  project_id          = var.project_id
  region              = var.region
  force_destroy       = false
  object_lock_enabled = true
  tags                = local.bucket_tags

  versioning {
    enabled = true
  }

  lifecycle {
    prevent_destroy = true
  }
}

resource "scaleway_object_bucket_lock_configuration" "archive" {
  bucket     = scaleway_object_bucket.archive.name
  project_id = var.project_id

  rule {
    default_retention {
      mode  = "COMPLIANCE"
      years = var.retention_years
    }
  }
}

resource "scaleway_object_bucket_policy" "writer" {
  bucket     = scaleway_object_bucket.archive.name
  project_id = var.project_id
  policy = jsonencode({
    Version = "2023-04-17"
    Id      = "nvbes-cross-account-audit-writer"
    Statement = [
      {
        Sid       = "WriteSignedAnchorsOnly"
        Effect    = "Allow"
        Action    = ["s3:PutObject"]
        Principal = { SCW = "application_id:${scaleway_iam_application.writer.id}" }
        Resource  = ["${scaleway_object_bucket.archive.name}/anchors/*"]
      }
    ]
  })
}
