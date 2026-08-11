# The email resources moved to the isolated email-production state. These
# declarations make an apply of the general production stack forget any legacy
# bindings without deleting live email infrastructure during the handoff.

removed {
  from = scaleway_sdb_sql_database.email
  lifecycle { destroy = false }
}

removed {
  from = scaleway_iam_application.email_database_runtime
  lifecycle { destroy = false }
}

removed {
  from = scaleway_iam_policy.email_database_runtime
  lifecycle { destroy = false }
}

removed {
  from = scaleway_iam_api_key.email_database_runtime
  lifecycle { destroy = false }
}

removed {
  from = scaleway_iam_application.email_database_migrator
  lifecycle { destroy = false }
}

removed {
  from = scaleway_iam_policy.email_database_migrator
  lifecycle { destroy = false }
}

removed {
  from = scaleway_iam_api_key.email_database_migrator
  lifecycle { destroy = false }
}

removed {
  from = scaleway_secret.email_database_migration_url
  lifecycle { destroy = false }
}

removed {
  from = scaleway_secret_version.email_database_migration_url
  lifecycle { destroy = false }
}

removed {
  from = scaleway_job_definition.email_database_migration
  lifecycle { destroy = false }
}

removed {
  from = scaleway_tem_domain.transactional
  lifecycle { destroy = false }
}

removed {
  from = cloudflare_dns_record.transactional_email_spf
  lifecycle { destroy = false }
}

removed {
  from = cloudflare_dns_record.transactional_email_dkim
  lifecycle { destroy = false }
}

removed {
  from = cloudflare_dns_record.transactional_email_dmarc
  lifecycle { destroy = false }
}

removed {
  from = cloudflare_dns_record.transactional_email_mx
  lifecycle { destroy = false }
}

removed {
  from = scaleway_tem_domain_validation.transactional
  lifecycle { destroy = false }
}

removed {
  from = scaleway_mnq_sns.email_events
  lifecycle { destroy = false }
}

removed {
  from = scaleway_mnq_sns_credentials.email_events_terraform
  lifecycle { destroy = false }
}

removed {
  from = scaleway_mnq_sns_topic.email_events
  lifecycle { destroy = false }
}

removed {
  from = scaleway_mnq_sns_topic_subscription.email_worker
  lifecycle { destroy = false }
}

removed {
  from = scaleway_tem_webhook.email_events
  lifecycle { destroy = false }
}

removed {
  from = scaleway_mnq_sqs.email_dispatch
  lifecycle { destroy = false }
}

removed {
  from = scaleway_mnq_sqs_credentials.email_dispatch_terraform
  lifecycle { destroy = false }
}

removed {
  from = scaleway_mnq_sqs_credentials.email_dispatch_publisher
  lifecycle { destroy = false }
}

removed {
  from = scaleway_mnq_sqs_credentials.email_dispatch_trigger
  lifecycle { destroy = false }
}

removed {
  from = scaleway_mnq_sqs_queue.email_dispatch_dead_letter
  lifecycle { destroy = false }
}

removed {
  from = scaleway_mnq_sqs_queue.email_dispatch
  lifecycle { destroy = false }
}

removed {
  from = scaleway_container_namespace.email_worker
  lifecycle { destroy = false }
}

removed {
  from = scaleway_container.email_runtime
  lifecycle { destroy = false }
}

removed {
  from = scaleway_container_trigger.email_dispatch
  lifecycle { destroy = false }
}

removed {
  from = scaleway_container_trigger.email_retention
  lifecycle { destroy = false }
}
