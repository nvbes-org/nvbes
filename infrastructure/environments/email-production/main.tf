locals {
  environment = "production"
  name_prefix = "nvbes-prod"

  tags = [
    "nvbes",
    "component:email",
    "environment:production",
    "managed-by:terraform",
  ]
}
