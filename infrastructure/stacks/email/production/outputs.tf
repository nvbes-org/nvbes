output "transactional_email_domain" {
  description = "Terraform-owned Scaleway TEM sender domain."
  value       = scaleway_tem_domain.transactional.name
}

output "transactional_email_domain_status" {
  description = "Current Scaleway TEM domain status."
  value       = scaleway_tem_domain.transactional.status
}

output "transactional_email_domain_validated" {
  description = "Whether Scaleway validated the Cloudflare-hosted email DNS records."
  value       = scaleway_tem_domain_validation.transactional.validated
}

output "transactional_email_event_topic_arn" {
  description = "SNS topic carrying signed Scaleway TEM lifecycle events."
  value       = scaleway_mnq_sns_topic.email_events.arn
}

output "transactional_email_event_subscription_arn" {
  description = "Confirmed HTTPS subscription receiving Scaleway TEM lifecycle events."
  value       = scaleway_mnq_sns_topic_subscription.email_worker.arn
}

output "transactional_email_webhook_id" {
  description = "Scaleway TEM webhook bound to the transactional sender domain."
  value       = scaleway_tem_webhook.email_events.id
}

output "email_worker_container_id" {
  description = "Terraform-owned production email-worker Serverless Container ID."
  value       = scaleway_container.email_worker.id
}

output "email_worker_public_endpoint" {
  description = "Public HTTPS endpoint multiplexing health checks, signed TEM webhooks and authenticated gRPC."
  value       = scaleway_container.email_worker.public_endpoint
}

output "email_worker_grpc_endpoint" {
  description = "Authenticated production gRPC endpoint shared with the Serverless HTTPS listener."
  value       = scaleway_container.email_worker.public_endpoint
}

output "email_worker_registry_endpoint" {
  description = "Private Scaleway registry namespace receiving signed email-worker releases."
  value       = scaleway_registry_namespace.email_worker.endpoint
}

output "email_worker_sentry_dsn_secret_id" {
  description = "Protected Secret Manager ID associated with the email-worker runtime binding."
  value       = scaleway_secret.email_worker_sentry_dsn.id
}

output "email_worker_sentry_dsn_secret_revision" {
  description = "Active Secret Manager revision triggering the email-worker rollout."
  value = scaleway_secret_version.email_worker_sentry_dsn[
    var.email_sentry_dsn_rotation.active_slot
  ].revision
}

output "email_worker_sentry_dsn_retained_revisions" {
  description = "Blue/green Secret Manager revisions retained for rollback during rotation."
  value = {
    for slot, secret_version in scaleway_secret_version.email_worker_sentry_dsn :
    slot => secret_version.revision
  }
}

output "email_worker_sentry_traces_sample_rate" {
  description = "Non-secret value injected as SENTRY_TRACES_SAMPLE_RATE."
  value       = var.email_sentry_traces_sample_rate
}

output "email_worker_sentry_runtime_binding" {
  description = "Non-secret runtime binding applied to the production email-worker Serverless Container."
  value = {
    container_id = scaleway_container.email_worker.id
    release      = local.email_worker_image_digest
    secret_id    = scaleway_secret.email_worker_sentry_dsn.id
    secret_revision = scaleway_secret_version.email_worker_sentry_dsn[
      var.email_sentry_dsn_rotation.active_slot
    ].revision
  }
}
