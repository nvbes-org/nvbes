locals {
  email_worker_image_digest = split("@", var.email_worker_source_image)[1]
  email_worker_image_tag    = replace(local.email_worker_image_digest, ":", "-")
  email_worker_registry_image = (
    "${scaleway_registry_namespace.email_worker.endpoint}/nvbes-email-worker:${local.email_worker_image_tag}"
  )
  email_worker_container_uuid = element(
    split("/", scaleway_container.email_worker.id),
    length(split("/", scaleway_container.email_worker.id)) - 1
  )
}

resource "scaleway_registry_namespace" "email_worker" {
  name        = "${local.name_prefix}-email-worker"
  description = "Private immutable images deployed to the production email-worker."
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  is_public   = false

  lifecycle {
    prevent_destroy = true
  }
}

# The signed GHCR release is mirrored only after Terraform owns the destination
# registry. Docker authentication is established by the protected deployment
# workflow; neither registry credential is persisted in Terraform artifacts.
resource "terraform_data" "email_worker_image" {
  triggers_replace = {
    source_image = var.email_worker_source_image
    target_image = local.email_worker_registry_image
  }

  provisioner "local-exec" {
    command = <<-EOT
      set -euo pipefail
      command -v docker >/dev/null 2>&1 || {
        echo "Docker with Buildx is required to mirror email-worker" >&2
        exit 1
      }
      docker buildx imagetools create \
        --tag "$EMAIL_WORKER_TARGET_IMAGE" \
        "$EMAIL_WORKER_SOURCE_IMAGE"
    EOT

    environment = {
      EMAIL_WORKER_SOURCE_IMAGE = var.email_worker_source_image
      EMAIL_WORKER_TARGET_IMAGE = local.email_worker_registry_image
    }
    interpreter = ["/usr/bin/env", "bash", "-c"]
  }
}

resource "scaleway_container_namespace" "email_worker" {
  name        = "${local.name_prefix}-email-worker"
  description = "Always-on production email delivery runtime."
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  tags        = concat(local.tags, ["service:email-worker"])

  lifecycle {
    prevent_destroy = true
  }
}

# The bootstrap command exposes only /health/live and does not read production
# secrets. Once this resource is ready, terraform_data.email_worker_runtime
# replaces the command and injects every secret in one Scaleway rollout.
resource "scaleway_container" "email_worker" {
  name         = "${local.name_prefix}-email-worker"
  description  = "Global transactional email dispatcher and signed TEM webhook."
  namespace_id = scaleway_container_namespace.email_worker.id
  region       = var.scaleway_region
  image        = local.email_worker_registry_image

  command                = ["/app/email-worker", "deployment-bootstrap"]
  port                   = 3040
  protocol               = "h2c"
  privacy                = "public"
  https_connections_only = true
  min_scale              = 1
  max_scale              = 1
  timeout                = 300
  tags                   = concat(local.tags, ["service:email-worker"])
  registry_sha256        = local.email_worker_image_digest

  environment_variables = {
    NVBES_ENVIRONMENT                  = local.environment
    NVBES_EMAIL_HTTP_BIND_ADDR         = "0.0.0.0:3040"
    NVBES_EMAIL_GRPC_BIND_ADDR         = "0.0.0.0:3040"
    NVBES_EMAIL_PROVIDER               = "scaleway"
    NVBES_EMAIL_FROM_EMAIL             = "no-reply@${local.transactional_email_domain}"
    NVBES_EMAIL_FROM_NAME              = "nvbes"
    NVBES_EMAIL_MESSAGE_ID_DOMAIN      = local.transactional_email_domain
    NVBES_EMAIL_PAYLOAD_RETENTION_DAYS = "30"
    NVBES_EMAIL_LEDGER_RETENTION_DAYS  = "400"
    NVBES_SCALEWAY_EMAIL_PROJECT_ID    = var.scaleway_project_id
    NVBES_SCALEWAY_EMAIL_REGION        = var.scaleway_region
    NVBES_EMAIL_SNS_TOPIC_ARN          = scaleway_mnq_sns_topic.email_events.arn
    NVBES_EMAIL_SNS_CA_BUNDLE_PATH     = "/etc/ssl/certs/ca-certificates.crt"
    SENTRY_RELEASE                     = local.email_worker_image_digest
    SENTRY_TRACES_SAMPLE_RATE          = tostring(var.email_sentry_traces_sample_rate)
  }

  startup_probe {
    failure_threshold = 12
    interval          = "5s"
    timeout           = "3s"

    http {
      path = "/health/live"
    }
  }

  liveness_probe {
    failure_threshold = 3
    interval          = "30s"
    timeout           = "5s"

    http {
      path = "/health/live"
    }
  }

  lifecycle {
    prevent_destroy = true

    # The write-only runtime adapter replaces the bootstrap command after the
    # first healthy deployment. Keeping it out of reconciliation prevents a
    # later Terraform refresh from restoring bootstrap mode.
    ignore_changes = [command]
  }

  depends_on = [terraform_data.email_worker_image]
}

# Serverless Containers cannot reference Secret Manager directly. Its native
# secret_environment_variables argument is not write-only, so assigning runtime
# secrets there would persist them in Terraform artifacts. This narrow adapter
# injects all secrets only after Terraform has created the container.
resource "terraform_data" "email_worker_runtime" {
  triggers_replace = {
    container_id           = scaleway_container.email_worker.id
    image_digest           = local.email_worker_image_digest
    runtime_secret_version = tostring(var.email_worker_runtime_secret_version)
    sample_rate            = tostring(var.email_sentry_traces_sample_rate)
    sentry_secret_id       = scaleway_secret.email_worker_sentry_dsn.id
    sentry_secret_revision = scaleway_secret_version.email_worker_sentry_dsn[
      var.email_sentry_dsn_rotation.active_slot
    ].revision
  }

  provisioner "local-exec" {
    command = <<-EOT
      set -euo pipefail
      command -v scw >/dev/null 2>&1 || {
        echo "Scaleway CLI (scw) is required to deploy email-worker" >&2
        exit 1
      }

      scw_command=(scw)
      if [[ -n "$SCW_PROFILE" ]]; then
        scw_command+=(--profile "$SCW_PROFILE")
      fi

      "$${scw_command[@]}" container container update "$EMAIL_WORKER_CONTAINER_ID" \
        "command.0=/app/email-worker" \
        "secret-environment-variables.NVBES_EMAIL_DATABASE_URL=$NVBES_EMAIL_DATABASE_URL" \
        "secret-environment-variables.NVBES_EMAIL_PRODUCER_TOKENS=$NVBES_EMAIL_PRODUCER_TOKENS" \
        "secret-environment-variables.NVBES_EMAIL_DATA_ENCRYPTION_KEY=$NVBES_EMAIL_DATA_ENCRYPTION_KEY" \
        "secret-environment-variables.NVBES_EMAIL_RECIPIENT_HMAC_KEY=$NVBES_EMAIL_RECIPIENT_HMAC_KEY" \
        "secret-environment-variables.NVBES_SCALEWAY_EMAIL_SECRET_KEY=$NVBES_SCALEWAY_EMAIL_SECRET_KEY" \
        "secret-environment-variables.NVBES_OBSERVABILITY_INTERNAL_TOKEN=$NVBES_OBSERVABILITY_INTERNAL_TOKEN" \
        "secret-environment-variables.SENTRY_DSN=$SENTRY_DSN" \
        region="$SCW_REGION" \
        --wait
    EOT

    environment = {
      EMAIL_WORKER_CONTAINER_ID          = local.email_worker_container_uuid
      NVBES_EMAIL_DATABASE_URL           = var.email_worker_database_url
      NVBES_EMAIL_PRODUCER_TOKENS        = var.email_worker_producer_tokens
      NVBES_EMAIL_DATA_ENCRYPTION_KEY    = var.email_worker_data_encryption_key
      NVBES_EMAIL_RECIPIENT_HMAC_KEY     = var.email_worker_recipient_hmac_key
      NVBES_SCALEWAY_EMAIL_SECRET_KEY    = var.email_worker_scaleway_secret_key
      NVBES_OBSERVABILITY_INTERNAL_TOKEN = var.email_worker_observability_internal_token
      SCW_PROFILE = (
        var.scaleway_production_profile == null
        ? ""
        : var.scaleway_production_profile
      )
      SCW_REGION = var.scaleway_region
      SENTRY_DSN = var.email_sentry_dsn
    }
    interpreter = ["/usr/bin/env", "bash", "-c"]
  }

  lifecycle {
    create_before_destroy = true
  }
}
