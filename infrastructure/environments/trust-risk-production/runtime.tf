resource "scaleway_registry_namespace" "trust_risk" {
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  name        = "${local.name_prefix}-trust-risk"
  description = "Private immutable deployment images for production Trust/Risk."
  is_public   = false

  lifecycle {
    prevent_destroy = true
  }
}

resource "scaleway_container_namespace" "trust_risk" {
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  name        = "${local.name_prefix}-trust-risk"
  description = "Independent production fraud, abuse, bot and risk engine."
  tags        = local.tags
}

locals {
  trust_risk_image_digest = split("@", var.trust_risk_image)[1]
  trust_risk_runtime_environment = {
    NVBES_ENVIRONMENT                           = local.environment
    NVBES_TRUST_RISK_BIND_ADDR                  = "0.0.0.0:8080"
    NVBES_TRUST_RISK_SIGNALS_RETENTION_DAYS     = "30"
    NVBES_TRUST_RISK_EVALUATIONS_RETENTION_DAYS = "400"
    NVBES_TRUST_RISK_LABELS_RETENTION_DAYS      = "400"
    NVBES_TRUST_RISK_REVIEWS_RETENTION_DAYS     = "400"
    NVBES_TRUST_RISK_AUDIT_RETENTION_DAYS       = "730"
    NVBES_OTLP_ENDPOINT                         = var.grafana_otlp_endpoint
    SENTRY_RELEASE                              = local.trust_risk_image_digest
    SENTRY_TRACES_SAMPLE_RATE                   = tostring(var.trust_risk_sentry_traces_sample_rate)
  }

  trust_risk_runtime_secrets = {
    NVBES_TRUST_RISK_DATABASE_URL      = local.trust_risk_database_runtime_url
    NVBES_TRUST_RISK_PRODUCER_POLICIES = var.trust_risk_producer_policies
    NVBES_TRUST_RISK_OPERATOR_TOKENS   = var.trust_risk_operator_tokens
    NVBES_TRUST_RISK_METRICS_TOKEN     = var.trust_risk_metrics_token
    NVBES_OTLP_AUTHORIZATION_HEADER    = var.grafana_otlp_authorization_header
    SENTRY_DSN                         = var.trust_risk_sentry_dsn
  }
}

resource "scaleway_container" "trust_risk" {
  namespace_id           = scaleway_container_namespace.trust_risk.id
  private_network_id     = try(length(var.private_network_id) > 0, false) ? var.private_network_id : null
  name                   = "${local.name_prefix}-trust-risk"
  description            = "Authenticated gRPC Trust/Risk engine and health endpoints."
  image                  = var.trust_risk_image
  privacy                = "public"
  port                   = 8080
  protocol               = "h2c"
  https_connections_only = true
  cpu_limit              = 560
  memory_limit_bytes     = 1073741824
  timeout                = 45
  min_scale              = 0
  max_scale              = 1

  environment_variables        = local.trust_risk_runtime_environment
  secret_environment_variables = local.trust_risk_runtime_secrets

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
    scaleway_iam_policy.trust_risk_database_runtime,
    scaleway_registry_namespace.trust_risk,
  ]
}
