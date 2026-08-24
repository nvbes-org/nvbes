locals {
  environment = "production"
  name_prefix = "nvbes-prod"

  tags = [
    "nvbes",
    "component:trust-risk",
    "environment:production",
    "managed-by:terraform",
  ]
}
