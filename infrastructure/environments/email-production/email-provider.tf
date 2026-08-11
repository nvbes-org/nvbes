locals {
  transactional_email_domain = "notify.${var.base_domain}"

  transactional_email_event_types = [
    "blocklist_created",
    "email_blocklisted",
    "email_deferred",
    "email_delivered",
    "email_dropped",
    "email_mailbox_not_found",
    "email_queued",
    "email_spam",
  ]
}

resource "scaleway_tem_domain" "transactional" {
  name       = local.transactional_email_domain
  project_id = var.scaleway_project_id
  region     = var.scaleway_region
  accept_tos = var.accept_scaleway_tem_terms
  autoconfig = false

  lifecycle {
    prevent_destroy = true
  }
}

resource "cloudflare_dns_record" "transactional_email_spf" {
  zone_id = var.cloudflare_zone_id
  name    = local.transactional_email_domain
  type    = "TXT"
  content = scaleway_tem_domain.transactional.spf_value
  ttl     = 300
  proxied = false
  comment = "Scaleway TEM SPF managed exclusively by Terraform."
}

resource "cloudflare_dns_record" "transactional_email_dkim" {
  zone_id = var.cloudflare_zone_id
  name    = trimsuffix(scaleway_tem_domain.transactional.dkim_name, ".")
  type    = "TXT"
  content = scaleway_tem_domain.transactional.dkim_config
  ttl     = 300
  proxied = false
  comment = "Scaleway TEM DKIM managed exclusively by Terraform."
}

resource "cloudflare_dns_record" "transactional_email_dmarc" {
  zone_id = var.cloudflare_zone_id
  name    = trimsuffix(scaleway_tem_domain.transactional.dmarc_name, ".")
  type    = "TXT"
  content = scaleway_tem_domain.transactional.dmarc_config
  ttl     = 300
  proxied = false
  comment = "Scaleway TEM DMARC managed exclusively by Terraform."
}

resource "cloudflare_dns_record" "transactional_email_mx" {
  zone_id  = var.cloudflare_zone_id
  name     = local.transactional_email_domain
  type     = "MX"
  content  = trimsuffix(scaleway_tem_domain.transactional.mx_blackhole, ".")
  priority = 10
  ttl      = 300
  proxied  = false
  comment  = "Scaleway TEM blackhole MX managed exclusively by Terraform."
}

resource "scaleway_tem_domain_validation" "transactional" {
  domain_id = scaleway_tem_domain.transactional.id
  region    = var.scaleway_region
  timeout   = 3600

  depends_on = [
    cloudflare_dns_record.transactional_email_dkim,
    cloudflare_dns_record.transactional_email_dmarc,
    cloudflare_dns_record.transactional_email_mx,
    cloudflare_dns_record.transactional_email_spf,
  ]
}

resource "scaleway_mnq_sns" "email_events" {
  project_id = var.scaleway_project_id
  region     = var.scaleway_region
}

resource "scaleway_mnq_sns_credentials" "email_events_terraform" {
  project_id = scaleway_mnq_sns.email_events.project_id
  region     = var.scaleway_region
  name       = "${local.name_prefix}-email-events-terraform"

  permissions {
    can_manage  = true
    can_publish = false
    can_receive = false
  }
}

resource "scaleway_mnq_sns_topic" "email_events" {
  project_id   = scaleway_mnq_sns.email_events.project_id
  region       = var.scaleway_region
  name         = "${local.name_prefix}-email-events"
  sns_endpoint = scaleway_mnq_sns.email_events.endpoint
  access_key   = scaleway_mnq_sns_credentials.email_events_terraform.access_key
  secret_key   = scaleway_mnq_sns_credentials.email_events_terraform.secret_key
}

resource "scaleway_mnq_sns_topic_subscription" "email_worker" {
  project_id = scaleway_mnq_sns.email_events.project_id
  region     = var.scaleway_region
  topic_id   = scaleway_mnq_sns_topic.email_events.id
  protocol   = "https"
  endpoint = coalesce(
    var.email_webhook_endpoint,
    "${scaleway_container.email_runtime["ingress"].public_endpoint}/webhooks/scaleway/topics-and-events",
  )
  sns_endpoint = scaleway_mnq_sns.email_events.endpoint
  access_key   = scaleway_mnq_sns_credentials.email_events_terraform.access_key
  secret_key   = scaleway_mnq_sns_credentials.email_events_terraform.secret_key
}

resource "scaleway_tem_webhook" "email_events" {
  name        = "${local.name_prefix}-email-events"
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  domain_id   = scaleway_tem_domain.transactional.id
  event_types = local.transactional_email_event_types
  sns_arn     = scaleway_mnq_sns_topic.email_events.arn

  depends_on = [scaleway_tem_domain_validation.transactional]
}
