resource "scaleway_registry_namespace" "worker" {
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  name        = "${local.name_prefix}-worker"
  description = "Private immutable deployment images for production Worker."
  is_public   = false

  lifecycle {
    prevent_destroy = true
  }
}

resource "scaleway_container_namespace" "worker" {
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  name        = "${local.name_prefix}-worker"
  description = "Production Worker serverless container namespace."
  tags        = local.tags
}

locals {
  worker_image_digest = try(split("@", var.worker_image)[1], "latest")
  worker_runtime_environment = {
    NVBES_ENVIRONMENT                     = local.environment
    NVBES_WORKER_PORT                     = "8081"
    NVBES_WORKER_DATABASE_MAX_CONNECTIONS = "5"
    NVBES_WORKER_DATABASE_MIN_CONNECTIONS = "0"
    NVBES_WORKER_POLL_INTERVAL_SECS       = "5"
    NVBES_WORKER_PRIVACY_INTERVAL_SECS    = "60"
    NVBES_WORKER_OTLP_ENDPOINT            = var.grafana_otlp_endpoint
    SENTRY_RELEASE                        = local.worker_image_digest
    SENTRY_TRACES_SAMPLE_RATE             = tostring(var.worker_sentry_traces_sample_rate)
  }
  worker_runtime_secrets = {
    DATABASE_URL                    = var.worker_database_url
    NVBES_WORKER_OTLP_AUTHORIZATION = var.grafana_otlp_authorization_header
    SENTRY_DSN                      = var.worker_sentry_dsn
  }
}

resource "scaleway_container" "worker" {
  namespace_id           = scaleway_container_namespace.worker.id
  private_network_id     = try(length(var.private_network_id) > 0, false) ? var.private_network_id : null
  name                   = "${local.name_prefix}-worker"
  description            = "Unified Background Worker serverless container."
  image                  = var.worker_image
  privacy                = "public"
  port                   = 8081
  protocol               = "http1"
  https_connections_only = true
  cpu_limit              = 560
  memory_limit_bytes     = 1073741824
  timeout                = 300
  min_scale              = 0
  max_scale              = 1

  environment_variables        = local.worker_runtime_environment
  secret_environment_variables = local.worker_runtime_secrets

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
    scaleway_registry_namespace.worker,
  ]
}
