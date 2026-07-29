terraform {
  required_version = ">= 1.11.0"

  required_providers {
    scaleway = {
      source  = "scaleway/scaleway"
      version = "~> 2.79"
    }
  }
}

provider "scaleway" {
  profile    = var.security_scaleway_profile
  project_id = var.security_project_id
  region     = var.scaleway_region
}
