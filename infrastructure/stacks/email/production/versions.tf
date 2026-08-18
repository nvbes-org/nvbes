terraform {
  required_version = ">= 1.11.0"

  required_providers {
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "~> 5.19"
    }
    grafana = {
      source  = "grafana/grafana"
      version = "~> 4.37"
    }
    scaleway = {
      source  = "scaleway/scaleway"
      version = "~> 2.79"
    }
  }
}
