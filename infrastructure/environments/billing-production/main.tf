provider "scaleway" {
  zone   = var.scaleway_zone
  region = var.scaleway_region
}

locals {
  name_prefix = "nvbes-${var.environment}"
  environment = var.environment
  tags = [
    "env:${var.environment}",
    "service:billing",
    "managed-by:terraform",
  ]
}
