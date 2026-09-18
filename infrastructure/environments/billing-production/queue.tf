resource "scaleway_mnq_sqs" "billing_dispatch" {
  project_id = var.scaleway_project_id
  region     = var.scaleway_region
}

resource "scaleway_mnq_sqs_credentials" "billing_dispatch_terraform" {
  project_id = scaleway_mnq_sqs.billing_dispatch.project_id
  region     = var.scaleway_region
  name       = "${local.name_prefix}-billing-dispatch-terraform"

  permissions {
    can_manage  = true
    can_publish = false
    can_receive = false
  }
}

resource "scaleway_mnq_sqs_credentials" "billing_dispatch_publisher" {
  project_id = scaleway_mnq_sqs.billing_dispatch.project_id
  region     = var.scaleway_region
  name       = "${local.name_prefix}-billing-dispatch-publisher"

  permissions {
    can_manage  = false
    can_publish = true
    can_receive = false
  }
}

resource "scaleway_mnq_sqs_credentials" "billing_dispatch_trigger" {
  project_id = scaleway_mnq_sqs.billing_dispatch.project_id
  region     = var.scaleway_region
  name       = "${local.name_prefix}-billing-dispatch-trigger"

  permissions {
    can_manage  = false
    can_publish = false
    can_receive = true
  }
}

resource "scaleway_mnq_sqs_queue" "billing_dispatch_dead_letter" {
  project_id      = scaleway_mnq_sqs.billing_dispatch.project_id
  region          = var.scaleway_region
  name            = "${local.name_prefix}-billing-dispatch-dlq"
  sqs_endpoint    = scaleway_mnq_sqs.billing_dispatch.endpoint
  access_key      = scaleway_mnq_sqs_credentials.billing_dispatch_terraform.access_key
  secret_key      = scaleway_mnq_sqs_credentials.billing_dispatch_terraform.secret_key
  message_max_age = 1209600
}

resource "scaleway_mnq_sqs_queue" "billing_dispatch" {
  project_id                 = scaleway_mnq_sqs.billing_dispatch.project_id
  region                     = var.scaleway_region
  name                       = "${local.name_prefix}-billing-dispatch"
  sqs_endpoint               = scaleway_mnq_sqs.billing_dispatch.endpoint
  access_key                 = scaleway_mnq_sqs_credentials.billing_dispatch_terraform.access_key
  secret_key                 = scaleway_mnq_sqs_credentials.billing_dispatch_terraform.secret_key
  visibility_timeout_seconds = 60
  message_max_age            = 86400

  dead_letter_queue {
    id                = scaleway_mnq_sqs_queue.billing_dispatch_dead_letter.id
    max_receive_count = 4
  }
}

resource "scaleway_container" "billing_worker" {
  namespace_id           = scaleway_container_namespace.billing.id
  private_network_id     = try(length(var.private_network_id) > 0, false) ? var.private_network_id : null
  name                   = "${local.name_prefix}-billing-worker"
  description            = "Scale-to-zero Billing background worker for outbox events, invoices, and reconciliation."
  image                  = coalesce(var.billing_worker_image, var.billing_image)
  privacy                = "private"
  port                   = 8080
  protocol               = "http1"
  https_connections_only = true
  cpu_limit              = 280
  memory_limit_bytes     = 536870912
  timeout                = 45
  min_scale              = 0
  max_scale              = 1

  environment_variables = merge(local.billing_runtime_environment, {
    NVBES_BILLING_QUEUE_ENDPOINT    = scaleway_mnq_sqs.billing_dispatch.endpoint
    NVBES_BILLING_QUEUE_URL         = scaleway_mnq_sqs_queue.billing_dispatch.url
    NVBES_BILLING_QUEUE_REGION      = var.scaleway_region
    NVBES_BILLING_QUEUE_ACCESS_KEY  = scaleway_mnq_sqs_credentials.billing_dispatch_publisher.access_key
    NVBES_BILLING_GRPC_ENDPOINT     = "https://${scaleway_container.billing.domain_name}"
  })

  secret_environment_variables = merge(local.billing_runtime_secrets, {
    NVBES_BILLING_QUEUE_SECRET_KEY  = scaleway_mnq_sqs_credentials.billing_dispatch_publisher.secret_key
    NVBES_BILLING_GRPC_AUTH_TOKEN   = var.billing_grpc_token
  })

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
    failure_threshold = 10
    interval          = "5s"
    timeout           = "1s"
  }

  depends_on = [
    scaleway_iam_policy.billing_database_runtime,
    scaleway_registry_namespace.billing,
  ]
}

resource "scaleway_container_trigger" "billing_dispatch" {
  container_id = scaleway_container.billing_worker.id
  name         = "${local.name_prefix}-billing-dispatch"

  destination_config {
    http_path   = "/internal/queue/billing-dispatch"
    http_method = "post"
  }

  sqs {
    endpoint   = scaleway_mnq_sqs.billing_dispatch.endpoint
    queue_url  = scaleway_mnq_sqs_queue.billing_dispatch.url
    access_key = scaleway_mnq_sqs_credentials.billing_dispatch_trigger.access_key
    secret_key = scaleway_mnq_sqs_credentials.billing_dispatch_trigger.secret_key
    project_id = var.scaleway_project_id
    region     = var.scaleway_region
  }
}

resource "scaleway_container_trigger" "billing_reconciliation" {
  container_id = scaleway_container.billing_worker.id
  name         = "${local.name_prefix}-billing-reconciliation"

  destination_config {
    http_path   = "/internal/reconciliation"
    http_method = "post"
  }

  cron {
    schedule = "0 4 * * *"
    timezone = "UTC"
  }
}
