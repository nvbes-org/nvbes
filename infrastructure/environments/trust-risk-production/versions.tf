terraform {
  required_version = ">= 1.15.5, < 1.16.0"

  required_providers {
    grafana = {
      source  = "grafana/grafana"
      version = "= 4.45.0"
    }
    scaleway = {
      source  = "scaleway/scaleway"
      version = "= 2.79.0"
    }
  }
}

provider "scaleway" {
  organization_id = var.scaleway_organization_id
  project_id      = var.scaleway_project_id
  region          = var.scaleway_region
  zone            = var.scaleway_zone
}

provider "grafana" {
  url  = var.grafana_url
  auth = var.grafana_service_account_token
}
