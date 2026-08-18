mock_provider "cloudflare" {}
mock_provider "grafana" {}
mock_provider "scaleway" {}

override_resource {
  target = scaleway_registry_namespace.email_worker
  values = {
    endpoint = "rg.fr-par.scw.cloud/nvbes-prod-email-worker"
  }
}

override_resource {
  target = scaleway_container.email_worker
  values = {
    id              = "fr-par/00000000-0000-4000-8000-000000000000"
    public_endpoint = "https://email-worker.functions.fnc.fr-par.scw.cloud"
  }
}

variables {
  accept_scaleway_tem_terms                 = true
  base_domain                               = "nvbes.eu"
  cloudflare_zone_id                        = "00000000000000000000000000000000"
  email_sentry_dsn                          = "https://public@example.invalid/1"
  email_worker_source_image                 = "ghcr.io/nvbes-org/nvbes-email-worker@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  email_worker_database_url                 = "postgres://email:secret@example.invalid/email"
  email_worker_producer_tokens              = "identity-service=identity-service-production-token"
  email_worker_data_encryption_key          = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="
  email_worker_recipient_hmac_key           = "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE="
  email_worker_scaleway_secret_key          = "test-scaleway-tem-secret"
  email_worker_observability_internal_token = "production-observability-token-value"
  grafana_email_contact_point               = "platform-oncall"
  grafana_prometheus_datasource_uid         = "grafanacloud-prom"
  grafana_service_account_token             = "test-grafana-token"
  grafana_url                               = "https://nvbes.grafana.net"
  scaleway_project_id                       = "00000000-0000-4000-8000-000000000001"
}

run "bootstrap_keeps_only_active_slot" {
  command = apply

  assert {
    condition = (
      output.email_worker_container_id == "fr-par/00000000-0000-4000-8000-000000000000" &&
      output.email_worker_public_endpoint == "https://email-worker.functions.fnc.fr-par.scw.cloud"
    )
    error_message = "Terraform must own and expose the production email-worker container."
  }

  assert {
    condition = (
      scaleway_mnq_sns_topic_subscription.email_worker.endpoint ==
      "https://email-worker.functions.fnc.fr-par.scw.cloud/webhooks/scaleway/topics-and-events"
    )
    error_message = "The SNS subscription endpoint must be derived from the Terraform-owned container."
  }

  assert {
    condition = (
      scaleway_container.email_worker.protocol == "h2c" &&
      scaleway_container.email_worker.environment_variables["NVBES_EMAIL_HTTP_BIND_ADDR"] == "0.0.0.0:3040" &&
      scaleway_container.email_worker.environment_variables["NVBES_EMAIL_GRPC_BIND_ADDR"] == "0.0.0.0:3040"
    )
    error_message = "HTTP and authenticated gRPC must share the single Serverless h2c port."
  }

  assert {
    condition = (
      length(scaleway_secret_version.email_worker_sentry_dsn) == 1 &&
      contains(keys(scaleway_secret_version.email_worker_sentry_dsn), "blue")
    )
    error_message = "Bootstrap must create only the active blue Sentry slot."
  }
}

run "rotation_retains_n_and_n_minus_one" {
  command = apply

  variables {
    email_sentry_dsn_rotation = {
      active_slot     = "green"
      blue_version    = 1
      green_version   = 2
      retain_previous = true
    }
  }

  assert {
    condition = (
      length(scaleway_secret_version.email_worker_sentry_dsn) == 2 &&
      contains(keys(scaleway_secret_version.email_worker_sentry_dsn), "blue") &&
      contains(keys(scaleway_secret_version.email_worker_sentry_dsn), "green")
    )
    error_message = "Rotation must retain both the active and previous Sentry slots."
  }

  assert {
    condition = (
      output.email_worker_sentry_dsn_retained_revisions["blue"] ==
      run.bootstrap_keeps_only_active_slot.email_worker_sentry_dsn_retained_revisions["blue"]
    )
    error_message = "The inactive blue slot must remain unchanged while green is rolled out."
  }
}

run "separate_apply_prunes_n_minus_one" {
  command = apply

  variables {
    email_sentry_dsn_rotation = {
      active_slot     = "green"
      blue_version    = 1
      green_version   = 2
      retain_previous = false
    }
  }

  assert {
    condition = (
      length(scaleway_secret_version.email_worker_sentry_dsn) == 1 &&
      contains(keys(scaleway_secret_version.email_worker_sentry_dsn), "green")
    )
    error_message = "The cleanup apply must retain only the active Sentry slot."
  }
}

run "rejects_non_monotonic_rotation" {
  command = plan

  variables {
    email_sentry_dsn_rotation = {
      active_slot     = "green"
      blue_version    = 2
      green_version   = 2
      retain_previous = true
    }
  }

  expect_failures = [var.email_sentry_dsn_rotation]
}

run "rejects_mutable_or_untrusted_image" {
  command = plan

  variables {
    email_worker_source_image = "ghcr.io/nvbes-org/nvbes-email-worker:latest"
  }

  expect_failures = [var.email_worker_source_image]
}
