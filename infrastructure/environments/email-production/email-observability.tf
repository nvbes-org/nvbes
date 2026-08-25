locals {
  email_alert_document = yamldecode(
    file("../../local/observability/grafana-provisioning/alerting/nvbes-email.yml")
  )
  email_alert_groups = {
    for group in local.email_alert_document.groups : group.name => group
  }
  email_dashboard_json = replace(
    file("../../local/observability/grafana-dashboards/nvbes-email-communications.json"),
    "\"uid\": \"Prometheus\"",
    "\"uid\": \"${var.grafana_prometheus_datasource_uid}\""
  )
}

resource "grafana_folder" "email_observability" {
  title = "nvbes-email"
}

resource "grafana_dashboard" "email_communications" {
  folder      = grafana_folder.email_observability.uid
  config_json = local.email_dashboard_json
  overwrite   = true
}

resource "grafana_rule_group" "email" {
  for_each = local.email_alert_groups

  name             = each.value.name
  folder_uid       = grafana_folder.email_observability.uid
  interval_seconds = 60
  org_id           = each.value.orgId

  dynamic "rule" {
    for_each = each.value.rules

    content {
      uid            = rule.value.uid
      name           = rule.value.title
      condition      = rule.value.condition
      for            = rule.value.for
      no_data_state  = rule.value.noDataState
      exec_err_state = rule.value.execErrState
      annotations    = rule.value.annotations
      labels         = rule.value.labels
      is_paused      = false

      notification_settings {
        contact_point   = var.grafana_email_contact_point
        group_by        = ["alertname", "grafana_folder", "severity"]
        group_wait      = "10s"
        group_interval  = "5m"
        repeat_interval = "1h"
      }

      dynamic "data" {
        for_each = rule.value.data

        content {
          ref_id = data.value.refId
          datasource_uid = data.value.datasourceUid == "__expr__" ? (
            "-100"
          ) : var.grafana_prometheus_datasource_uid
          query_type = try(data.value.model.queryType, "")
          model = jsonencode(merge(data.value.model, {
            datasource = merge(try(data.value.model.datasource, {}), {
              uid = data.value.datasourceUid == "__expr__" ? "__expr__" : var.grafana_prometheus_datasource_uid
            })
          }))

          relative_time_range {
            from = try(data.value.relativeTimeRange.from, 600)
            to   = try(data.value.relativeTimeRange.to, 0)
          }
        }
      }
    }
  }
}
