terraform {
  required_version = ">= 1.11.0"

  required_providers {
    github = {
      source  = "integrations/github"
      version = "= 6.13.0"
    }
    scaleway = {
      source  = "scaleway/scaleway"
      version = "= 2.80.0"
    }
    time = {
      source  = "hashicorp/time"
      version = "= 0.14.1"
    }
  }
}
