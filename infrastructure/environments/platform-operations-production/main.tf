locals {
  environment = "production"
  name_prefix = "nvbes-prod"
  tags = [
    "nvbes",
    "component:platform-operations",
    "environment:production",
    "managed-by:terraform",
  ]
}
