resource "scaleway_container_namespace" "platform_operations" {
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  name        = "${local.name_prefix}-platform-ops"
  description = "Minimal solo operator cockpit for nvbes V1."
  tags        = local.tags
}

locals {
  platform_operations_runtime_environment = {
    NVBES_ENVIRONMENT                        = local.environment
    NVBES_PLATFORM_OPERATIONS_PORT           = "8084"
    NVBES_PLATFORM_OPERATIONS_PUBLIC_KEY_PEM = var.platform_operations_public_key_pem
    NVBES_PLATFORM_OPERATIONS_ISSUER         = var.platform_operations_issuer
    NVBES_PLATFORM_OPERATIONS_SERVICES       = var.platform_operations_services
  }
  platform_operations_runtime_secrets = {
    NVBES_PLATFORM_OPERATIONS_DATABASE_URL = var.platform_operations_database_url
  }
}

resource "scaleway_container" "platform_operations" {
  namespace_id           = scaleway_container_namespace.platform_operations.id
  private_network_id     = try(length(var.private_network_id) > 0, false) ? var.private_network_id : null
  name                   = "${local.name_prefix}-platform-operations"
  description            = "Solo operator cockpit runtime with scale-to-zero."
  image                  = var.platform_operations_image
  privacy                = "private"
  port                   = 8084
  protocol               = "http1"
  https_connections_only = true
  cpu_limit              = 560
  memory_limit_bytes     = 1073741824
  timeout                = 45
  min_scale              = 0
  max_scale              = 1

  environment_variables        = local.platform_operations_runtime_environment
  secret_environment_variables = local.platform_operations_runtime_secrets

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
}
