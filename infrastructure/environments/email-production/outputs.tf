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

output "email_worker_endpoint" {
  description = "Public HTTP/2 endpoint for authenticated gRPC and signed TEM webhooks."
  value       = scaleway_container.email_runtime["ingress"].public_endpoint
}

output "email_registry_endpoint" {
  description = "Private Scaleway Container Registry receiving verified email-worker images."
  value       = scaleway_registry_namespace.email_worker.endpoint
}

output "email_dispatch_queue_url" {
  description = "SQS queue receiving opaque email ledger UUIDs."
  value       = scaleway_mnq_sqs_queue.email_dispatch.url
}

output "email_dispatch_dead_letter_queue_url" {
  description = "SQS dead-letter queue for messages that exhaust trigger delivery."
  value       = scaleway_mnq_sqs_queue.email_dispatch_dead_letter.url
}

output "email_database_id" {
  description = "Dedicated scale-to-zero Serverless SQL database used by email-worker."
  value       = scaleway_sdb_sql_database.email.id
}

output "email_database_endpoint" {
  description = "Non-secret endpoint of the dedicated email Serverless SQL database."
  value       = scaleway_sdb_sql_database.email.endpoint
}

output "email_database_migration_job_id" {
  description = "Job definition that applies email-worker SQLx migrations."
  value       = scaleway_job_definition.email_database_migration.id
}
