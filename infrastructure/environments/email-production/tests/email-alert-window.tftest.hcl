mock_provider "cloudflare" {}
mock_provider "grafana" {}
mock_provider "scaleway" {}

variables {
  accept_scaleway_tem_terms          = true
  base_domain                        = "nvbes.eu"
  cloudflare_zone_id                 = "00000000000000000000000000000000"
  scaleway_project_id                = "00000000-0000-4000-8000-000000000001"
  email_worker_image                 = "rg.fr-par.scw.cloud/test/nvbes-email-worker@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  email_producer_tokens              = "test-producer-token"
  email_data_encryption_key          = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="
  email_recipient_hmac_key           = "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE="
  email_observability_internal_token = "test-observability-token-at-least-32-characters"
  email_sentry_dsn                   = "https://public@example.invalid/1"
  scaleway_email_secret_key          = "test-tem-secret"
  email_sns_ca_bundle_pem            = "test-ca-bundle"
  grafana_email_contact_point        = "grafana-default-email"
  grafana_prometheus_datasource_uid  = "grafanacloud-prom"
  grafana_url                        = "https://example.grafana.net"
}

run "closed_window_does_not_report_missing_samples_as_queue_staleness" {
  command = plan

  plan_options {
    target = [grafana_rule_group.email]
  }

  assert {
    condition = alltrue([
      for rule in grafana_rule_group.email["nvbes-email"].rule :
      rule.no_data_state == "OK" && rule.exec_err_state == "Error"
    ])
    error_message = "Missing idle metrics must be OK while query errors remain detectable."
  }

  assert {
    condition = alltrue([
      for rule in grafana_rule_group.email["nvbes-email"].rule :
      rule.for == "5m" && rule.is_paused == false && alltrue([
        for data in rule.data :
        data.ref_id != "C" || jsondecode(data.model).conditions[0].evaluator.params[0] == 300
      ]) if rule.uid == "nvbes-email-queue-stale"
    ])
    error_message = "An observed queue deadline breach must retain its threshold and evaluation."
  }
}

run "open_window_detects_missing_queue_telemetry" {
  command = plan

  variables {
    email_internal_validation_enabled = true
  }

  plan_options {
    target = [grafana_rule_group.email]
  }

  assert {
    condition = alltrue([
      for rule in grafana_rule_group.email["nvbes-email"].rule :
      rule.no_data_state == (rule.uid == "nvbes-email-queue-stale" ? "Alerting" : "OK") &&
      rule.exec_err_state == "Error"
    ])
    error_message = "Active validation must restore missing-queue-telemetry detection."
  }
}
