resource "scaleway_registry_namespace" "account" {
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  name        = "${local.name_prefix}-account"
  description = "Private immutable deployment images for production Account."
  is_public   = false

  lifecycle {
    prevent_destroy = true
  }
}

resource "scaleway_container_namespace" "account" {
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  name        = "${local.name_prefix}-account"
  description = "Closed production Account foundation."
  tags        = local.tags
}

locals {
  account_image_digest = split("@", var.account_image)[1]
  account_runtime_environment = {
    NVBES_ENVIRONMENT                       = local.environment
    NVBES_ACCOUNT_BIND_ADDR                 = "0.0.0.0:8080"
    NVBES_ACCOUNT_DATABASE_MAX_CONNECTIONS  = "5"
    NVBES_IDENTITY_TOKEN_ISSUER             = var.identity_token_issuer
    NVBES_ACCOUNT_TOKEN_AUDIENCE            = var.identity_token_audience
    NVBES_IDENTITY_TOKEN_KEY_ID             = var.identity_token_key_id
    NVBES_OTLP_ENDPOINT                     = var.grafana_otlp_endpoint
    SENTRY_RELEASE                          = local.account_image_digest
    SENTRY_TRACES_SAMPLE_RATE               = tostring(var.account_sentry_traces_sample_rate)
  }
  account_runtime_secrets = {
    NVBES_ACCOUNT_DATABASE_URL              = local.account_database_runtime_url
    NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM     = var.identity_token_public_key_pem
    NVBES_ACCOUNT_METRICS_TOKEN             = var.account_metrics_token
    NVBES_OTLP_AUTHORIZATION_HEADER         = var.grafana_otlp_authorization_header
    SENTRY_DSN                              = var.account_sentry_dsn
  }
}

resource "scaleway_container" "account" {
  namespace_id           = scaleway_container_namespace.account.id
  private_network_id     = try(length(var.private_network_id) > 0, false) ? var.private_network_id : null
  name                   = "${local.name_prefix}-account"
  description            = "Closed Account foundation with health and protected metrics only."
  image                  = var.account_image
  privacy                = "public"
  port                   = 8080
  protocol               = "http1"
  https_connections_only = true
  cpu_limit              = 560
  memory_limit_bytes     = 1073741824
  timeout                = 45
  min_scale              = 0
  max_scale              = 1

  environment_variables        = local.account_runtime_environment
  secret_environment_variables = local.account_runtime_secrets

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
    scaleway_iam_policy.account_database_runtime,
    scaleway_registry_namespace.account,
  ]
}

resource "scaleway_job_definition" "account_database_migration" {
  name                   = "${local.name_prefix}-account-database-migration"
  description            = "Explicit Account schema migration run before runtime deployment."
  project_id             = var.scaleway_project_id
  region                 = var.scaleway_region
  cpu_limit              = 560
  memory_limit           = 1024
  local_storage_capacity = 1024
  image_uri              = var.account_image
  args                   = ["migrate"]

  secret_reference {
    secret_id   = scaleway_secret.account_database_migration_url.id
    environment = "NVBES_ACCOUNT_DATABASE_URL"
  }

  depends_on = [
    scaleway_secret_version.account_database_migration_url,
  ]
}

resource "scaleway_job_definition" "account_synthetic_smoke" {
  name                   = "${local.name_prefix}-account-synthetic-smoke"
  description            = "Proves Account lifecycle, privacy, teams, and outbox without public exposure."
  project_id             = var.scaleway_project_id
  region                 = var.scaleway_region
  cpu_limit              = 560
  memory_limit           = 1024
  local_storage_capacity = 1024
  image_uri              = var.account_image
  args                   = ["synthetic-account-smoke"]

  secret_reference {
    secret_id   = scaleway_secret.account_database_runtime_url.id
    environment = "NVBES_ACCOUNT_DATABASE_URL"
  }

  depends_on = [
    scaleway_secret_version.account_database_runtime_url,
  ]
}

resource "scaleway_job_definition" "account_privacy_jobs" {
  name                   = "${local.name_prefix}-account-privacy-jobs"
  description            = "Processes pending Account data exports, expiration, and cancellable closures."
  project_id             = var.scaleway_project_id
  region                 = var.scaleway_region
  cpu_limit              = 560
  memory_limit           = 1024
  local_storage_capacity = 1024
  image_uri              = var.account_image
  args                   = ["process-privacy-jobs"]

  secret_reference {
    secret_id   = scaleway_secret.account_database_runtime_url.id
    environment = "NVBES_ACCOUNT_DATABASE_URL"
  }

  depends_on = [
    scaleway_secret_version.account_database_runtime_url,
  ]
}
