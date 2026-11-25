locals {
  environment = "production"
  name_prefix = "nvbes-prod"
  tags = [
    "nvbes",
    "component:api",
    "environment:production",
    "managed-by:terraform",
  ]
}
