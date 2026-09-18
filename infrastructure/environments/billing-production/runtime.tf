resource "scaleway_registry_namespace" "billing" {
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  name        = "${local.name_prefix}-billing"
  description = "Private immutable deployment images for production Billing."
  is_public   = false

  lifecycle {
    prevent_destroy = true
  }
}

resource "scaleway_container_namespace" "billing" {
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  name        = "${local.name_prefix}-billing"
  description = "Independent production Billing engine and Stripe test intake."
  tags        = local.tags
}

locals {
  billing_image_digest = split("@", var.billing_image)[1]
  billing_runtime_environment = {
    NVBES_ENVIRONMENT         = local.environment
    NVBES_BILLING_BIND_ADDR   = "0.0.0.0:8080"
    NVBES_APP_URL             = var.app_url
    NVBES_OTLP_ENDPOINT       = var.grafana_otlp_endpoint
    SENTRY_RELEASE            = local.billing_image_digest
    SENTRY_TRACES_SAMPLE_RATE = tostring(var.billing_sentry_traces_sample_rate)
  }

  billing_runtime_secrets = {
    NVBES_BILLING_DATABASE_URL      = local.billing_database_runtime_url
    NVBES_STRIPE_SECRET_KEY         = var.stripe_secret_key
    NVBES_STRIPE_WEBHOOK_SECRET     = var.stripe_webhook_secret
    NVBES_BILLING_METRICS_TOKEN     = var.billing_metrics_token
    NVBES_BILLING_OPERATOR_TOKEN    = var.billing_operator_token
    NVBES_IDENTITY_PUBLIC_KEY_PEM   = var.identity_public_key_pem
    NVBES_OTLP_AUTHORIZATION_HEADER = var.grafana_otlp_authorization_header
    SENTRY_DSN                      = var.billing_sentry_dsn
  }
}

resource "scaleway_container" "billing" {
  namespace_id           = scaleway_container_namespace.billing.id
  private_network_id     = try(length(var.private_network_id) > 0, false) ? var.private_network_id : null
  name                   = "${local.name_prefix}-billing"
  description            = "Isolated Billing service engine and Stripe test webhook intake."
  image                  = var.billing_image
  privacy                = "public"
  port                   = 8080
  protocol               = "h2c"
  https_connections_only = true
  cpu_limit              = 560
  memory_limit_bytes     = 1073741824
  timeout                = 45
  min_scale              = 0
  max_scale              = 1

  environment_variables        = local.billing_runtime_environment
  secret_environment_variables = local.billing_runtime_secrets

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
    scaleway_iam_policy.billing_database_runtime,
    scaleway_registry_namespace.billing,
  ]
}
