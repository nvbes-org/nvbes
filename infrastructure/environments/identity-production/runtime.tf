resource "scaleway_registry_namespace" "identity" {
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  name        = "${local.name_prefix}-identity"
  description = "Private immutable deployment images for production Identity."
  is_public   = false

  lifecycle {
    prevent_destroy = true
  }
}

resource "scaleway_container_namespace" "identity" {
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  name        = "${local.name_prefix}-identity"
  description = "Closed production Identity foundation."
  tags        = local.tags
}

locals {
  identity_image_digest = split("@", var.identity_image)[1]
  identity_runtime_environment = {
    NVBES_ENVIRONMENT                       = local.environment
    NVBES_IDENTITY_BIND_ADDR                = "0.0.0.0:8080"
    NVBES_IDENTITY_DATABASE_MAX_CONNECTIONS = "5"
    NVBES_IDENTITY_MFA_KEY_VERSION          = tostring(var.identity_mfa_key_version)
    NVBES_OTLP_ENDPOINT                     = var.grafana_otlp_endpoint
    SENTRY_RELEASE                          = local.identity_image_digest
    SENTRY_TRACES_SAMPLE_RATE               = tostring(var.identity_sentry_traces_sample_rate)
  }
  identity_runtime_secrets = {
    NVBES_IDENTITY_DATABASE_URL       = local.identity_database_runtime_url
    NVBES_IDENTITY_MFA_ENCRYPTION_KEY = var.identity_mfa_encryption_key
    NVBES_IDENTITY_METRICS_TOKEN      = var.identity_metrics_token
    NVBES_OTLP_AUTHORIZATION_HEADER   = var.grafana_otlp_authorization_header
    SENTRY_DSN                        = var.identity_sentry_dsn
  }
}

resource "scaleway_container" "identity" {
  namespace_id           = scaleway_container_namespace.identity.id
  private_network_id     = try(length(var.private_network_id) > 0, false) ? var.private_network_id : null
  name                   = "${local.name_prefix}-identity"
  description            = "Closed Identity foundation with health and protected metrics only."
  image                  = var.identity_image
  privacy                = "public"
  port                   = 8080
  protocol               = "http1"
  https_connections_only = true
  cpu_limit              = 560
  memory_limit_bytes     = 1073741824
  timeout                = 45
  min_scale              = 0
  max_scale              = 1

  environment_variables        = local.identity_runtime_environment
  secret_environment_variables = local.identity_runtime_secrets

  liveness_probe {
    http {
      path = "/health/live"
    }
    failure_threshold = 3
    interval          = "30s"
    timeout           = "5s"
  }

  startup_probe {
    http {
      path = "/health/ready"
    }
    failure_threshold = 12
    interval          = "5s"
    timeout           = "2s"
  }

  depends_on = [
    scaleway_iam_policy.identity_database_runtime,
    scaleway_registry_namespace.identity,
  ]
}

resource "scaleway_job_definition" "identity_database_migration" {
  name                   = "${local.name_prefix}-identity-database-migration"
  description            = "Explicit Identity schema migration run before runtime deployment."
  project_id             = var.scaleway_project_id
  region                 = var.scaleway_region
  cpu_limit              = 560
  memory_limit           = 1024
  local_storage_capacity = 1024
  image_uri              = var.identity_image
  args                   = ["migrate"]

  secret_reference {
    secret_id   = scaleway_secret.identity_database_migration_url.id
    environment = "NVBES_IDENTITY_DATABASE_URL"
  }

  depends_on = [
    scaleway_iam_policy.identity_database_migrator,
    scaleway_registry_namespace.identity,
    scaleway_secret_version.identity_database_migration_url,
  ]
}

resource "scaleway_secret" "identity_synthetic_password" {
  name        = "${local.name_prefix}-identity-synthetic-password"
  description = "Initial password used only by the explicit Identity synthetic job."
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  protected   = true
  tags        = local.tags
}

resource "scaleway_secret_version" "identity_synthetic_password" {
  secret_id   = scaleway_secret.identity_synthetic_password.id
  region      = var.scaleway_region
  data        = var.identity_synthetic_password
  description = "Terraform-managed Identity synthetic password."
}

resource "scaleway_secret" "identity_synthetic_recovered_password" {
  name        = "${local.name_prefix}-identity-synthetic-recovered-password"
  description = "Recovery password used only by the explicit Identity synthetic job."
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  protected   = true
  tags        = local.tags
}

resource "scaleway_secret_version" "identity_synthetic_recovered_password" {
  secret_id   = scaleway_secret.identity_synthetic_recovered_password.id
  region      = var.scaleway_region
  data        = var.identity_synthetic_recovered_password
  description = "Terraform-managed Identity synthetic recovery password."
}

resource "scaleway_job_definition" "identity_synthetic_auth" {
  name                   = "${local.name_prefix}-identity-synthetic-auth"
  description            = "Explicit non-delivering authentication and recovery production proof."
  project_id             = var.scaleway_project_id
  region                 = var.scaleway_region
  cpu_limit              = 560
  memory_limit           = 1024
  local_storage_capacity = 1024
  image_uri              = var.identity_image
  args                   = ["synthetic-auth-smoke"]

  secret_reference {
    secret_id   = scaleway_secret.identity_database_runtime_url.id
    environment = "NVBES_IDENTITY_DATABASE_URL"
  }

  secret_reference {
    secret_id   = scaleway_secret.identity_synthetic_password.id
    environment = "NVBES_IDENTITY_SYNTHETIC_PASSWORD"
  }

  secret_reference {
    secret_id   = scaleway_secret.identity_synthetic_recovered_password.id
    environment = "NVBES_IDENTITY_SYNTHETIC_RECOVERED_PASSWORD"
  }

  depends_on = [
    scaleway_registry_namespace.identity,
    scaleway_secret_version.identity_database_runtime_url,
    scaleway_secret_version.identity_synthetic_password,
    scaleway_secret_version.identity_synthetic_recovered_password,
  ]
}
