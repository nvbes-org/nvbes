terraform {
  required_version = ">= 1.15.5, < 1.16.0"

  required_providers {
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "= 5.19.0"
    }
    scaleway = {
      source  = "scaleway/scaleway"
      version = "= 2.79.0"
    }
  }
}

provider "scaleway" {
  project_id = var.scaleway_project_id
  region     = var.scaleway_region
  zone       = var.scaleway_zone
}

provider "cloudflare" {}
