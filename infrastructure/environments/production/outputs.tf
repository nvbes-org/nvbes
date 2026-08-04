output "api_domain" {
  value = local.api_domain
}

output "private_network_id" {
  value = module.scaleway.private_network_id
}

output "audit_archive_bucket_name" {
  value = module.scaleway.audit_archive_bucket_name
}

output "audit_anchor_signer_access_key" {
  value     = module.scaleway.audit_anchor_signer_access_key
  sensitive = true
}

output "audit_anchor_signer_secret_key" {
  value     = module.scaleway.audit_anchor_signer_secret_key
  sensitive = true
}

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
