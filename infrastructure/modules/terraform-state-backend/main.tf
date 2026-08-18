locals {
  bucket_tags = merge(
    { for tag in var.tags : replace(tag, ":", "_") => tag },
    { data_class = "terraform_state" },
  )

  state_paths = {
    for stack in var.state_stacks : stack => {
      prefix   = "${var.environment}/${stack}"
      state    = "${var.environment}/${stack}/terraform.tfstate"
      lockfile = "${var.environment}/${stack}/terraform.tfstate.tflock"
    }
  }

  tls_condition = {
    Bool = {
      "aws:SecureTransport" = "true"
    }
  }
}

resource "scaleway_object_bucket" "terraform_state" {
  name          = var.bucket_name
  project_id    = var.project_id
  region        = var.region
  force_destroy = false
  tags          = local.bucket_tags

  versioning {
    enabled = true
  }

  lifecycle {
    prevent_destroy = true
  }
}

resource "scaleway_object_bucket_server_side_encryption_configuration" "terraform_state" {
  bucket = scaleway_object_bucket.terraform_state.name
  region = var.region

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

resource "scaleway_iam_application" "state" {
  for_each = var.state_stacks

  name        = "nvbes-${var.environment}-terraform-${each.key}"
  description = "Terraform state identity restricted to the ${var.environment}/${each.key} prefix."
}

resource "scaleway_iam_policy" "state" {
  for_each = var.state_stacks

  name           = "nvbes-${var.environment}-terraform-${each.key}"
  description    = "Object Storage permissions intersected with the ${each.key} state bucket policy."
  application_id = scaleway_iam_application.state[each.key].id

  rule {
    project_ids = [var.project_id]
    permission_set_names = [
      "ObjectStorageBucketsRead",
      "ObjectStorageObjectsDelete",
      "ObjectStorageObjectsRead",
      "ObjectStorageObjectsWrite",
    ]
  }
}

resource "scaleway_iam_api_key" "state" {
  for_each = var.state_stacks

  application_id     = scaleway_iam_application.state[each.key].id
  default_project_id = var.project_id
  description        = "Rotating Terraform state credential for ${var.environment}/${each.key}."
}

resource "scaleway_object_bucket_policy" "terraform_state" {
  bucket     = scaleway_object_bucket.terraform_state.name
  project_id = var.project_id
  policy = jsonencode({
    Version = "2023-04-17"
    Id      = "nvbes-${var.environment}-terraform-state"
    Statement = concat(
      [
        for stack, path in local.state_paths : {
          Sid       = "List${title(replace(stack, "-", " "))}StatePrefix"
          Effect    = "Allow"
          Principal = { SCW = "application_id:${scaleway_iam_application.state[stack].id}" }
          Action    = ["s3:ListBucket"]
          Resource  = [scaleway_object_bucket.terraform_state.name]
          Condition = merge(local.tls_condition, {
            StringLike = {
              "s3:prefix" = ["${path.prefix}/*"]
            }
          })
        }
      ],
      [
        for stack, path in local.state_paths : {
          Sid       = "ReadWrite${title(replace(stack, "-", " "))}State"
          Effect    = "Allow"
          Principal = { SCW = "application_id:${scaleway_iam_application.state[stack].id}" }
          Action    = ["s3:GetObject", "s3:PutObject"]
          Resource  = ["${scaleway_object_bucket.terraform_state.name}/${path.state}"]
          Condition = local.tls_condition
        }
      ],
      [
        for stack, path in local.state_paths : {
          Sid       = "Manage${title(replace(stack, "-", " "))}StateLock"
          Effect    = "Allow"
          Principal = { SCW = "application_id:${scaleway_iam_application.state[stack].id}" }
          Action    = ["s3:DeleteObject", "s3:GetObject", "s3:PutObject"]
          Resource  = ["${scaleway_object_bucket.terraform_state.name}/${path.lockfile}"]
          Condition = local.tls_condition
        }
      ],
    )
  })
}
