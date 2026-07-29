locals {
  environment = "production"
  name_prefix = "nvbes-prod"
  api_domain  = "api.${var.base_domain}"
  web_domain  = "app.${var.base_domain}"

  tags = [
    "nvbes",
    "environment:production",
    "managed-by:terraform",
  ]

  cloudflare_origin_ipv4_cidrs = [
    "173.245.48.0/20",
    "103.21.244.0/22",
    "103.22.200.0/22",
    "103.31.4.0/22",
    "141.101.64.0/18",
    "108.162.192.0/18",
    "190.93.240.0/20",
    "188.114.96.0/20",
    "197.234.240.0/22",
    "198.41.128.0/17",
    "162.158.0.0/15",
    "104.16.0.0/13",
    "104.24.0.0/14",
    "172.64.0.0/13",
    "131.0.72.0/22",
  ]
}

module "scaleway" {
  source = "../../modules/scaleway-v1"

  project_id                     = var.scaleway_project_id
  region                         = var.scaleway_region
  zone                           = var.scaleway_zone
  environment                    = local.environment
  name_prefix                    = local.name_prefix
  private_subnet                 = "10.40.0.0/22"
  enable_jit_ssh                 = var.enable_jit_ssh
  ssh_allowed_ips                = var.ssh_allowed_ips
  edge_allowed_ipv4_cidrs        = local.cloudflare_origin_ipv4_cidrs
  api_instance_type              = "PRO2-S"
  worker_instance_type           = "PRO2-XS"
  rdb_node_type                  = "DB-PRO2-M"
  postgres_user                  = "nvbes"
  postgres_password              = var.postgres_password
  postgres_backup_retention_days = 30
  bucket_name                    = var.files_bucket_name
  bucket_cors_allowed_origins    = ["https://${local.web_domain}"]
  tags                           = local.tags

  enable_external_audit_archive         = true
  audit_archive_bucket_name             = var.audit_archive_bucket_name
  audit_archive_writer_access_key       = var.audit_archive_writer_access_key
  audit_anchor_kms_key_id               = var.audit_anchor_kms_key_id
  audit_anchor_kms_auth_token_secret_id = var.audit_anchor_kms_auth_token_secret_id
  audit_archive_writer_secret_id        = var.audit_archive_writer_secret_id
  account_database_url_secret_id        = var.account_database_url_secret_id
}

resource "cloudflare_dns_record" "api" {
  zone_id = var.cloudflare_zone_id
  name    = local.api_domain
  type    = "A"
  content = module.scaleway.api_public_ip
  ttl     = 1
  proxied = true
}

resource "cloudflare_dns_record" "web" {
  zone_id = var.cloudflare_zone_id
  name    = local.web_domain
  type    = "CNAME"
  content = "placeholder.pages.dev"
  ttl     = 1
  proxied = true
}

resource "cloudflare_ruleset" "managed_waf" {
  zone_id     = var.cloudflare_zone_id
  name        = "nvbes-production-managed-waf"
  description = "Cloudflare managed and OWASP protections for nvbes production."
  kind        = "zone"
  phase       = "http_request_firewall_managed"

  rules = [
    {
      ref         = "execute_cloudflare_managed_ruleset"
      description = "Execute the Cloudflare Managed Ruleset."
      expression  = "true"
      action      = "execute"
      action_parameters = {
        id = "efb7b8c949ac4650a09736fc376e9aee"
      }
    },
    {
      ref         = "execute_cloudflare_owasp_ruleset"
      description = "Execute the Cloudflare OWASP Core Ruleset."
      expression  = "true"
      action      = "execute"
      action_parameters = {
        id = "4814384a9e5d4991b9815dcfc25d2f1f"
      }
    },
  ]
}

resource "cloudflare_ruleset" "custom_firewall" {
  zone_id = var.cloudflare_zone_id
  name    = "nvbes-production-custom-firewall"
  kind    = "zone"
  phase   = "http_request_firewall_custom"

  rules = [
    {
      ref         = "block_non_standard_ports"
      description = "Block HTTP traffic outside ports 80 and 443."
      expression  = "(not cf.edge.server_port in {80 443})"
      action      = "block"
    },
    {
      ref         = "challenge_high_risk_auth_traffic"
      description = "Challenge hostile traffic targeting identity endpoints."
      expression  = "(http.host eq \"${local.api_domain}\" and http.request.uri.path matches \"^/(login|oauth|v1/auth)/\")"
      action      = "managed_challenge"
    },
  ]
}

locals {
  zone_settings = {
    ssl                      = "strict"
    min_tls_version          = "1.3"
    tls_1_3                  = "zrt"
    http2                    = "on"
    http3                    = "on"
    origin_max_http_version  = "2"
    always_use_https         = "on"
    automatic_https_rewrites = "on"
  }
}

resource "cloudflare_zone_setting" "settings" {
  for_each   = local.zone_settings
  zone_id    = var.cloudflare_zone_id
  setting_id = each.key
  value      = each.value
}

resource "cloudflare_zone_dnssec" "dnssec" {
  zone_id = var.cloudflare_zone_id
}
