locals {
  credential_slots = {
    a = 0
    b = floor(var.credential_rotation_days / 2)
  }

  active_credential_slot = timecmp(
    time_rotating.ci_cache["a"].rotation_rfc3339,
    time_rotating.ci_cache["b"].rotation_rfc3339,
  ) > 0 ? "a" : "b"

  bucket_tags = merge(
    { for tag in var.tags : replace(tag, ":", "_") => tag },
    { data_class = "reproducible_ci_cache" },
  )
}

resource "scaleway_account_project" "ci_cache" {
  name            = var.project_name
  description     = "Isolated, disposable build caches for nvbes CI."
  organization_id = var.organization_id

  lifecycle {
    prevent_destroy = true
  }
}

resource "scaleway_registry_namespace" "ci_cache" {
  project_id  = scaleway_account_project.ci_cache.id
  region      = var.region
  name        = var.registry_name
  description = "Private BuildKit layer cache; never a deployment image source."
  is_public   = false

  lifecycle {
    prevent_destroy = true
  }
}

resource "scaleway_object_bucket" "ci_cache" {
  name          = var.bucket_name
  project_id    = scaleway_account_project.ci_cache.id
  region        = var.region
  force_destroy = false
  tags          = local.bucket_tags

  lifecycle_rule {
    id                                     = "expire-reproducible-cache"
    enabled                                = true
    abort_incomplete_multipart_upload_days = 1

    expiration {
      days = var.object_retention_days
    }
  }

  lifecycle {
    prevent_destroy = true
  }
}

resource "scaleway_object_bucket_server_side_encryption_configuration" "ci_cache" {
  bucket     = scaleway_object_bucket.ci_cache.name
  project_id = scaleway_account_project.ci_cache.id
  region     = var.region

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

resource "scaleway_iam_application" "ci_cache" {
  name            = "nvbes-production-ci-cache"
  description     = "GitHub Actions identity restricted to the isolated CI cache project."
  organization_id = var.organization_id

  lifecycle {
    prevent_destroy = true
  }
}

resource "scaleway_iam_policy" "ci_cache" {
  name            = "nvbes-production-ci-cache"
  description     = "BuildKit registry and reproducible object-cache access only."
  application_id  = scaleway_iam_application.ci_cache.id
  organization_id = var.organization_id

  rule {
    project_ids = [scaleway_account_project.ci_cache.id]
    permission_set_names = [
      "ContainerRegistryFullAccess",
      "ObjectStorageBucketsRead",
      "ObjectStorageObjectsDelete",
      "ObjectStorageObjectsRead",
      "ObjectStorageObjectsWrite",
    ]
  }
}

resource "scaleway_iam_application" "branch_cache" {
  name            = "nvbes-branch-ci-cache"
  description     = "GitHub Actions identity restricted to isolated branch compiler caches."
  organization_id = var.organization_id

  lifecycle {
    prevent_destroy = true
  }
}

resource "scaleway_iam_policy" "branch_cache" {
  name            = "nvbes-branch-ci-cache"
  description     = "Read and write access to isolated branch compiler-cache prefixes only."
  application_id  = scaleway_iam_application.branch_cache.id
  organization_id = var.organization_id

  rule {
    project_ids = [scaleway_account_project.ci_cache.id]
    permission_set_names = [
      "ObjectStorageBucketsRead",
      "ObjectStorageObjectsRead",
      "ObjectStorageObjectsWrite",
    ]
  }
}

resource "scaleway_object_bucket_policy" "ci_cache" {
  bucket     = scaleway_object_bucket.ci_cache.name
  project_id = scaleway_account_project.ci_cache.id
  depends_on = [scaleway_object_bucket_server_side_encryption_configuration.ci_cache]
  policy = jsonencode({
    Version = "2023-04-17"
    Id      = "nvbes-production-ci-cache"
    Statement = [
      {
        Sid       = "ReadBucketConfigurationForAuthorizedPrincipals"
        Effect    = "Allow"
        Principal = "*"
        Action = [
          "s3:GetBucketAcl",
          "s3:GetBucketCORS",
          "s3:GetEncryptionConfiguration",
          "s3:GetBucketObjectLockConfiguration",
          "s3:GetBucketTagging",
          "s3:GetBucketVersioning",
          "s3:GetLifecycleConfiguration",
          "s3:ListBucket",
          "s3:PutEncryptionConfiguration",
        ]
        Resource = [scaleway_object_bucket.ci_cache.name]
        Condition = {
          Bool = {
            "aws:SecureTransport" = "true"
          }
        }
      },
      {
        Sid       = "ListCacheObjects"
        Effect    = "Allow"
        Principal = { SCW = "application_id:${scaleway_iam_application.ci_cache.id}" }
        Action    = ["s3:ListBucket"]
        Resource  = [scaleway_object_bucket.ci_cache.name]
        Condition = {
          Bool = {
            "aws:SecureTransport" = "true"
          }
        }
      },
      {
        Sid       = "ReadWriteTrustedCacheObjects"
        Effect    = "Allow"
        Principal = { SCW = "application_id:${scaleway_iam_application.ci_cache.id}" }
        Action    = ["s3:GetObject", "s3:PutObject"]
        Resource  = ["${scaleway_object_bucket.ci_cache.name}/trusted/*"]
        Condition = {
          Bool = {
            "aws:SecureTransport" = "true"
          }
        }
      },
      {
        Sid       = "ListBranchCacheObjects"
        Effect    = "Allow"
        Principal = { SCW = "application_id:${scaleway_iam_application.branch_cache.id}" }
        Action    = ["s3:ListBucket"]
        Resource  = [scaleway_object_bucket.ci_cache.name]
        Condition = {
          Bool = {
            "aws:SecureTransport" = "true"
          }
        }
      },
      {
        Sid       = "ReadWriteBranchCacheObjects"
        Effect    = "Allow"
        Principal = { SCW = "application_id:${scaleway_iam_application.branch_cache.id}" }
        Action    = ["s3:GetObject", "s3:PutObject"]
        Resource  = ["${scaleway_object_bucket.ci_cache.name}/branches/*"]
        Condition = {
          Bool = {
            "aws:SecureTransport" = "true"
          }
        }
      },
    ]
  })
}

resource "time_rotating" "ci_cache" {
  for_each = local.credential_slots

  rfc3339       = timeadd(var.credential_rotation_epoch, format("%dh", each.value * 24))
  rotation_days = var.credential_rotation_days
}

resource "scaleway_iam_api_key" "ci_cache" {
  for_each = local.credential_slots

  application_id     = scaleway_iam_application.ci_cache.id
  default_project_id = scaleway_account_project.ci_cache.id
  description        = "Rotating nvbes CI cache credential slot ${upper(each.key)}."
  expires_at = timeadd(
    time_rotating.ci_cache[each.key].rotation_rfc3339,
    format("%dh", var.credential_expiry_grace_days * 24),
  )

  lifecycle {
    create_before_destroy = true
  }

  depends_on = [scaleway_iam_policy.ci_cache]
}

resource "time_rotating" "branch_cache" {
  for_each = local.credential_slots

  rfc3339       = timeadd(var.credential_rotation_epoch, format("%dh", each.value * 24))
  rotation_days = var.credential_rotation_days
}

resource "scaleway_iam_api_key" "branch_cache" {
  for_each = local.credential_slots

  application_id     = scaleway_iam_application.branch_cache.id
  default_project_id = scaleway_account_project.ci_cache.id
  description        = "Rotating nvbes branch CI cache credential slot ${upper(each.key)}."
  expires_at = timeadd(
    time_rotating.branch_cache[each.key].rotation_rfc3339,
    format("%dh", var.credential_expiry_grace_days * 24),
  )

  lifecycle {
    create_before_destroy = true
  }

  depends_on = [scaleway_iam_policy.branch_cache]
}

resource "github_repository_environment" "ci_cache" {
  repository  = var.github_repository
  environment = var.github_environment

  deployment_branch_policy {
    protected_branches     = false
    custom_branch_policies = true
  }

  lifecycle {
    prevent_destroy = true
  }
}

resource "github_repository_environment_deployment_policy" "main" {
  repository     = var.github_repository
  environment    = github_repository_environment.ci_cache.environment
  branch_pattern = "main"
}

resource "github_repository_environment_deployment_policy" "dev" {
  repository     = var.github_repository
  environment    = github_repository_environment.ci_cache.environment
  branch_pattern = "dev"
}

resource "github_repository_environment_deployment_policy" "staging" {
  repository     = var.github_repository
  environment    = github_repository_environment.ci_cache.environment
  branch_pattern = "staging"
}

resource "github_repository_environment_deployment_policy" "release" {
  repository     = var.github_repository
  environment    = github_repository_environment.ci_cache.environment
  branch_pattern = "release/*"
}

resource "github_actions_environment_secret" "access_key" {
  repository  = var.github_repository
  environment = github_repository_environment.ci_cache.environment
  secret_name = "SCW_CI_CACHE_ACCESS_KEY"
  value       = scaleway_iam_api_key.ci_cache[local.active_credential_slot].access_key
}

resource "github_actions_environment_secret" "secret_key" {
  repository  = var.github_repository
  environment = github_repository_environment.ci_cache.environment
  secret_name = "SCW_CI_CACHE_SECRET_KEY"
  value       = scaleway_iam_api_key.ci_cache[local.active_credential_slot].secret_key
}

resource "github_actions_environment_variable" "registry_endpoint" {
  repository    = var.github_repository
  environment   = github_repository_environment.ci_cache.environment
  variable_name = "SCW_CI_CACHE_REGISTRY_ENDPOINT"
  value         = scaleway_registry_namespace.ci_cache.endpoint
}

resource "github_actions_environment_variable" "bucket_name" {
  repository    = var.github_repository
  environment   = github_repository_environment.ci_cache.environment
  variable_name = "SCW_CI_CACHE_BUCKET"
  value         = scaleway_object_bucket.ci_cache.name
}

resource "github_actions_environment_variable" "s3_endpoint" {
  repository    = var.github_repository
  environment   = github_repository_environment.ci_cache.environment
  variable_name = "SCW_CI_CACHE_S3_ENDPOINT"
  value         = "https://s3.${var.region}.scw.cloud"
}

resource "github_actions_environment_variable" "region" {
  repository    = var.github_repository
  environment   = github_repository_environment.ci_cache.environment
  variable_name = "SCW_CI_CACHE_REGION"
  value         = var.region
}

resource "github_repository_environment" "branch_cache" {
  repository  = var.github_repository
  environment = var.github_branch_environment

  lifecycle {
    prevent_destroy = true
  }
}

resource "github_actions_environment_secret" "branch_access_key" {
  repository  = var.github_repository
  environment = github_repository_environment.branch_cache.environment
  secret_name = "SCW_CI_CACHE_ACCESS_KEY"
  value       = scaleway_iam_api_key.branch_cache[local.active_credential_slot].access_key
}

resource "github_actions_environment_secret" "branch_secret_key" {
  repository  = var.github_repository
  environment = github_repository_environment.branch_cache.environment
  secret_name = "SCW_CI_CACHE_SECRET_KEY"
  value       = scaleway_iam_api_key.branch_cache[local.active_credential_slot].secret_key
}

resource "github_actions_environment_variable" "branch_bucket_name" {
  repository    = var.github_repository
  environment   = github_repository_environment.branch_cache.environment
  variable_name = "SCW_CI_CACHE_BUCKET"
  value         = scaleway_object_bucket.ci_cache.name
}

resource "github_actions_environment_variable" "branch_s3_endpoint" {
  repository    = var.github_repository
  environment   = github_repository_environment.branch_cache.environment
  variable_name = "SCW_CI_CACHE_S3_ENDPOINT"
  value         = "https://s3.${var.region}.scw.cloud"
}

resource "github_actions_environment_variable" "branch_region" {
  repository    = var.github_repository
  environment   = github_repository_environment.branch_cache.environment
  variable_name = "SCW_CI_CACHE_REGION"
  value         = var.region
}
