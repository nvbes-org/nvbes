terraform {
  required_version = ">= 1.11.0"

  required_providers {
    github = {
      source = "integrations/github"
    }
    scaleway = {
      source = "scaleway/scaleway"
    }
    time = {
      source = "hashicorp/time"
    }
  }
}
