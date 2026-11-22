resource "scaleway_registry_namespace" "api" {
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  name        = "${local.name_prefix}-api"
  description = "Private immutable deployment images for production API."
  is_public   = false

  lifecycle {
    prevent_destroy = true
  }
}

resource "scaleway_container_namespace" "api" {
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  name        = "${local.name_prefix}-api"
  description = "Production API serverless container namespace."
  tags        = local.tags
}

locals {
  api_image_digest = try(split("@", var.api_image)[1], "latest")
  api_runtime_environment = {
    NVBES_ENVIRONMENT                  = local.environment
    NVBES_API_PORT                     = "8080"
    NVBES_API_DATABASE_MAX_CONNECTIONS = "10"
    NVBES_API_DATABASE_MIN_CONNECTIONS = "0"
    NVBES_API_OTLP_ENDPOINT            = var.grafana_otlp_endpoint
    SENTRY_RELEASE                     = local.api_image_digest
    SENTRY_TRACES_SAMPLE_RATE          = tostring(var.api_sentry_traces_sample_rate)
  }
  api_runtime_secrets = {
    DATABASE_URL                 = local.api_database_runtime_url
    NVBES_API_OTLP_AUTHORIZATION = var.grafana_otlp_authorization_header
    SENTRY_DSN                   = var.api_sentry_dsn
  }
}

resource "scaleway_container" "api" {
  namespace_id           = scaleway_container_namespace.api.id
  private_network_id     = try(length(var.private_network_id) > 0, false) ? var.private_network_id : null
  name                   = "${local.name_prefix}-api"
  description            = "Modular Monolith API serverless container."
  image                  = var.api_image
  privacy                = "public"
  port                   = 8080
  protocol               = "http1"
  https_connections_only = true
  cpu_limit              = 560
  memory_limit_bytes     = 1073741824
  timeout                = 45
  min_scale              = 0
  max_scale              = 1

  environment_variables        = local.api_runtime_environment
  secret_environment_variables = local.api_runtime_secrets

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
    scaleway_iam_policy.api_database_runtime,
    scaleway_registry_namespace.api,
  ]
}
