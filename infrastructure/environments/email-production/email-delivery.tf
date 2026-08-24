resource "scaleway_mnq_sqs" "email_dispatch" {
  project_id = var.scaleway_project_id
  region     = var.scaleway_region
}

resource "scaleway_mnq_sqs_credentials" "email_dispatch_terraform" {
  project_id = scaleway_mnq_sqs.email_dispatch.project_id
  region     = var.scaleway_region
  name       = "${local.name_prefix}-email-dispatch-terraform"

  permissions {
    can_manage  = true
    can_publish = false
    can_receive = false
  }
}

resource "scaleway_mnq_sqs_credentials" "email_dispatch_publisher" {
  project_id = scaleway_mnq_sqs.email_dispatch.project_id
  region     = var.scaleway_region
  name       = "${local.name_prefix}-email-dispatch-publisher"

  permissions {
    can_manage  = false
    can_publish = true
    can_receive = false
  }
}

resource "scaleway_mnq_sqs_credentials" "email_dispatch_trigger" {
  project_id = scaleway_mnq_sqs.email_dispatch.project_id
  region     = var.scaleway_region
  name       = "${local.name_prefix}-email-dispatch-trigger"

  permissions {
    can_manage  = false
    can_publish = false
    can_receive = true
  }
}

resource "scaleway_mnq_sqs_queue" "email_dispatch_dead_letter" {
  project_id      = scaleway_mnq_sqs.email_dispatch.project_id
  region          = var.scaleway_region
  name            = "${local.name_prefix}-email-dispatch-dlq"
  sqs_endpoint    = scaleway_mnq_sqs.email_dispatch.endpoint
  access_key      = scaleway_mnq_sqs_credentials.email_dispatch_terraform.access_key
  secret_key      = scaleway_mnq_sqs_credentials.email_dispatch_terraform.secret_key
  message_max_age = 1209600
}

resource "scaleway_mnq_sqs_queue" "email_dispatch" {
  project_id                 = scaleway_mnq_sqs.email_dispatch.project_id
  region                     = var.scaleway_region
  name                       = "${local.name_prefix}-email-dispatch"
  sqs_endpoint               = scaleway_mnq_sqs.email_dispatch.endpoint
  access_key                 = scaleway_mnq_sqs_credentials.email_dispatch_terraform.access_key
  secret_key                 = scaleway_mnq_sqs_credentials.email_dispatch_terraform.secret_key
  visibility_timeout_seconds = 60
  message_max_age            = 86400

  dead_letter_queue {
    id                = scaleway_mnq_sqs_queue.email_dispatch_dead_letter.id
    max_receive_count = 4
  }
}

resource "scaleway_registry_namespace" "email_worker" {
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  name        = "${local.name_prefix}-email-worker"
  description = "Private immutable deployment images for production email runtimes."
  is_public   = false

  lifecycle {
    prevent_destroy = true
  }
}

resource "scaleway_container_namespace" "email_worker" {
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  name        = "${local.name_prefix}-email-worker"
  description = "Scale-to-zero transactional email acceptance and dispatch."
  tags        = local.tags
}

locals {
  email_runtime_roles = {
    ingress = {
      privacy     = "public"
      description = "Authenticated gRPC acceptance and signed TEM webhooks."
    }
    dispatch = {
      privacy     = "private"
      description = "Private SQS dispatch and retention triggers."
    }
  }

  email_runtime_environment = {
    NVBES_ENVIRONMENT                  = local.environment
    NVBES_EMAIL_FROM_EMAIL             = "no-reply@${local.transactional_email_domain}"
    NVBES_EMAIL_FROM_NAME              = "nvbes"
    NVBES_EMAIL_MESSAGE_ID_DOMAIN      = local.transactional_email_domain
    NVBES_EMAIL_PROVIDER               = "scaleway"
    NVBES_SCALEWAY_EMAIL_PROJECT_ID    = var.scaleway_project_id
    NVBES_SCALEWAY_EMAIL_REGION        = var.scaleway_region
    NVBES_EMAIL_QUEUE_ENDPOINT         = scaleway_mnq_sqs.email_dispatch.endpoint
    NVBES_EMAIL_QUEUE_URL              = scaleway_mnq_sqs_queue.email_dispatch.url
    NVBES_EMAIL_QUEUE_REGION           = var.scaleway_region
    NVBES_EMAIL_QUEUE_ACCESS_KEY       = scaleway_mnq_sqs_credentials.email_dispatch_publisher.access_key
    NVBES_EMAIL_SNS_TOPIC_ARN          = scaleway_mnq_sns_topic.email_events.arn
    NVBES_EMAIL_PAYLOAD_RETENTION_DAYS = "30"
    NVBES_EMAIL_LEDGER_RETENTION_DAYS  = "400"
  }

  email_runtime_secrets = {
    NVBES_EMAIL_DATABASE_URL           = local.email_database_runtime_url
    NVBES_EMAIL_PRODUCER_TOKENS        = var.email_producer_tokens
    NVBES_EMAIL_DATA_ENCRYPTION_KEY    = var.email_data_encryption_key
    NVBES_EMAIL_RECIPIENT_HMAC_KEY     = var.email_recipient_hmac_key
    NVBES_OBSERVABILITY_INTERNAL_TOKEN = var.email_observability_internal_token
    NVBES_SCALEWAY_EMAIL_SECRET_KEY    = var.scaleway_email_secret_key
    NVBES_EMAIL_QUEUE_SECRET_KEY       = scaleway_mnq_sqs_credentials.email_dispatch_publisher.secret_key
    NVBES_EMAIL_SNS_CA_BUNDLE_PEM      = var.email_sns_ca_bundle_pem
  }
}

resource "scaleway_container" "email_runtime" {
  for_each = local.email_runtime_roles

  namespace_id           = scaleway_container_namespace.email_worker.id
  private_network_id     = try(length(var.private_network_id) > 0, false) ? var.private_network_id : null
  name                   = "${local.name_prefix}-email-${each.key}"
  description            = each.value.description
  image                  = var.email_worker_image
  privacy                = each.value.privacy
  port                   = 8080
  protocol               = "h2c"
  https_connections_only = true
  cpu_limit              = 280
  memory_limit_bytes     = 536870912
  timeout                = 45
  min_scale              = 0
  max_scale              = 10

  environment_variables = merge(local.email_runtime_environment, {
    NVBES_EMAIL_RUNTIME_ROLE = each.key
  })

  secret_environment_variables = local.email_runtime_secrets

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
    scaleway_iam_policy.email_database_runtime,
    scaleway_registry_namespace.email_worker,
  ]
}

resource "scaleway_container_trigger" "email_dispatch" {
  container_id = scaleway_container.email_runtime["dispatch"].id
  name         = "${local.name_prefix}-email-dispatch"

  destination_config {
    http_path   = "/internal/queue/email-dispatch"
    http_method = "post"
  }

  sqs {
    endpoint   = scaleway_mnq_sqs.email_dispatch.endpoint
    queue_url  = scaleway_mnq_sqs_queue.email_dispatch.url
    access_key = scaleway_mnq_sqs_credentials.email_dispatch_trigger.access_key
    secret_key = scaleway_mnq_sqs_credentials.email_dispatch_trigger.secret_key
    project_id = var.scaleway_project_id
    region     = var.scaleway_region
  }
}

resource "scaleway_container_trigger" "email_retention" {
  container_id = scaleway_container.email_runtime["dispatch"].id
  name         = "${local.name_prefix}-email-retention"

  destination_config {
    http_path   = "/internal/retention"
    http_method = "post"
  }

  cron {
    schedule = "15 3 * * *"
    timezone = "UTC"
  }
}
