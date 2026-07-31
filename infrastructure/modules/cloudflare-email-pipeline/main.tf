terraform {
  required_version = ">= 1.5.0"
  required_providers {
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "~> 4.0"
    }
    scaleway = {
      source  = "scaleway/scaleway"
      version = "~> 2.30"
    }
  }
}

# Domaine d'envoi Scaleway Transactional Email (Paris)
resource "scaleway_tem_domain" "main" {
  name       = var.domain_name
  project_id = var.scaleway_project_id
  region     = "fr-par"
}

# Cloudflare Worker Script pour le dispatch d'emails
resource "cloudflare_worker_script" "email_worker" {
  account_id = var.cloudflare_account_id
  name       = "nvbes-email-worker-${var.environment}"
  content    = file("${path.module}/../../../apps/email-worker/dist/index.js")
  module     = true

  plain_text_binding {
    name = "ENVIRONMENT"
    text = var.environment
  }

  secret_text_binding {
    name = "SCALEWAY_SECRET_KEY"
    text = var.scaleway_secret_key
  }

  secret_text_binding {
    name = "SCALEWAY_PROJECT_ID"
    text = var.scaleway_project_id
  }
}

# Route Cloudflare Worker pour écouter sur /api/v1/email/send
resource "cloudflare_worker_route" "email_worker_route" {
  zone_id     = var.cloudflare_zone_id
  pattern     = "email-${var.environment}.${var.domain_name}/*"
  script_name = cloudflare_worker_script.email_worker.name
}
