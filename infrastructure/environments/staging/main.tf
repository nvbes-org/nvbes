locals {
  environment = "staging"
  name_prefix = "nvbes-stg"
  domain      = "staging.${var.base_domain}"
  api_domain  = "api.staging.${var.base_domain}"
  web_domain  = "app.staging.${var.base_domain}"

  tags = [
    "nvbes",
    "environment:staging",
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

  secret_inventory = {
    "NVBES_DATABASE_URL"              = "PostgreSQL private connection string for the API and workers."
    "NVBES_STRIPE_SECRET_KEY"         = "Staging Stripe secret key."
    "NVBES_STRIPE_WEBHOOK_SECRET"     = "Staging Stripe webhook signing secret."
    "NVBES_OBJECT_STORAGE_ACCESS_KEY" = "Runtime IAM access key for Object Storage."
    "NVBES_OBJECT_STORAGE_SECRET_KEY" = "Runtime IAM secret key for Object Storage."
    "NVBES_AUTH_PEPPER"               = "Application-level auth pepper."
  }
}

module "scaleway" {
  source = "../../modules/scaleway-v1"

  project_id                     = var.scaleway_project_id
  region                         = var.scaleway_region
  zone                           = var.scaleway_zone
  environment                    = local.environment
  name_prefix                    = local.name_prefix
  private_subnet                 = "10.30.0.0/22"
  ssh_allowed_ips                = var.ssh_allowed_ips
  enable_jit_ssh                 = false
  edge_allowed_ipv4_cidrs        = local.cloudflare_origin_ipv4_cidrs
  api_instance_type              = "DEV1-M"
  worker_instance_type           = "DEV1-S"
  rdb_node_type                  = "DB-DEV-S"
  postgres_user                  = "nvbes"
  postgres_password              = var.postgres_password
  postgres_backup_retention_days = 7
  bucket_name                    = "nvbes-staging-files"
  bucket_cors_allowed_origins    = ["https://${local.web_domain}"]
  tags                           = local.tags
}

resource "cloudflare_ruleset" "managed_waf" {
  zone_id     = var.cloudflare_zone_id
  name        = "nvbes-staging-managed-waf"
  description = "Cloudflare managed protections for nvbes staging."
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

resource "cloudflare_ruleset" "zone_custom_firewall" {
  zone_id = var.cloudflare_zone_id
  name    = "nvbes-staging-custom-firewall"
  kind    = "zone"
  phase   = "http_request_firewall_custom"

  rules = [
    {
      ref         = "block_non_standard_ports_staging"
      description = "Block HTTP traffic outside ports 80 and 443."
      expression  = "(not cf.edge.server_port in {80 443})"
      action      = "block"
    },
    {
      ref         = "rate_limit_public_api_staging"
      description = "Challenge excessive staging API traffic."
      expression  = "(http.host eq \"${local.api_domain}\" and http.request.uri.path starts_with \"/v1/\")"
      action      = "managed_challenge"
    }
  ]
}

# Zone settings — each setting is an individual resource in Cloudflare provider v5.
# Protocol negotiation priority (client → Cloudflare edge):
#   HTTP/3 (QUIC) → HTTP/2 (h2 ALPN) → HTTP/1.1 (fallback)
# Origin connection: HTTP/2 when TLS is enabled on the backend, else HTTP/1.1

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
    opportunistic_encryption = "on"
  }
}

resource "cloudflare_zone_setting" "settings" {
  for_each   = local.zone_settings
  zone_id    = var.cloudflare_zone_id
  setting_id = each.key
  value      = each.value
}

resource "cloudflare_zone_setting" "security_header" {
  zone_id    = var.cloudflare_zone_id
  setting_id = "security_header"
  value = jsonencode({
    strict_transport_security = {
      enabled            = true
      max_age            = 31536000
      include_subdomains = true
      preload            = true
      nosniff            = true
    }
  })
}

resource "cloudflare_dns_record" "caa_letsencrypt" {
  zone_id = var.cloudflare_zone_id
  name    = "@"
  type    = "CAA"
  ttl     = 1
  data = {
    flags = 0
    tag   = "issue"
    value = "letsencrypt.org"
  }
}

resource "cloudflare_dns_record" "caa_digicert" {
  zone_id = var.cloudflare_zone_id
  name    = "@"
  type    = "CAA"
  ttl     = 1
  data = {
    flags = 0
    tag   = "issue"
    value = "digicert.com"
  }
}

resource "cloudflare_dns_record" "caa_comodoca" {
  zone_id = var.cloudflare_zone_id
  name    = "@"
  type    = "CAA"
  ttl     = 1
  data = {
    flags = 0
    tag   = "issue"
    value = "comodoca.com"
  }
}

resource "cloudflare_dns_record" "caa_globalsign" {
  zone_id = var.cloudflare_zone_id
  name    = "@"
  type    = "CAA"
  ttl     = 1
  data = {
    flags = 0
    tag   = "issue"
    value = "globalsign.com"
  }
}

resource "cloudflare_dns_record" "caa_issuewild_none" {
  zone_id = var.cloudflare_zone_id
  name    = "@"
  type    = "CAA"
  ttl     = 1
  data = {
    flags = 0
    tag   = "issuewild"
    value = ";"
  }
}

resource "cloudflare_dns_record" "caa_iodef" {
  zone_id = var.cloudflare_zone_id
  name    = "@"
  type    = "CAA"
  ttl     = 1
  data = {
    flags = 0
    tag   = "iodef"
    value = "mailto:security@${var.base_domain}"
  }
}

resource "cloudflare_zone_dnssec" "dnssec" {
  zone_id = var.cloudflare_zone_id
}
