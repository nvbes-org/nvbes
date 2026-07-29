locals {
  security_alert_documents = [
    yamldecode(file("../../local/observability/grafana-provisioning/alerting/nvbes-security-identity.yml")),
    yamldecode(file("../../local/observability/grafana-provisioning/alerting/nvbes-security-data.yml")),
  ]

  security_alert_groups = {
    for group in flatten([
      for document in local.security_alert_documents : document.groups
    ]) : group.name => group
  }
}

resource "grafana_folder" "security_alerts" {
  title = "nvbes-security"
}

resource "grafana_rule_group" "security" {
  for_each = local.security_alert_groups

  name             = each.value.name
  folder_uid       = grafana_folder.security_alerts.uid
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
        contact_point   = var.grafana_security_contact_point
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
          ) : var.grafana_loki_datasource_uid
          query_type = try(data.value.model.queryType, "")
          model      = jsonencode(data.value.model)

          relative_time_range {
            from = try(data.value.relativeTimeRange.from, 600)
            to   = try(data.value.relativeTimeRange.to, 0)
          }
        }
      }
    }
  }
}
