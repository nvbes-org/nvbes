locals {
  environment = "production"
  name_prefix = "nvbes-prod"
  tags = [
    "nvbes",
    "component:identity",
    "environment:production",
    "managed-by:terraform",
  ]
}
