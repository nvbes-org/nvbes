locals {
  environment = "production"
  name_prefix = "nvbes-prod"
  tags = [
    "nvbes",
    "component:account",
    "environment:production",
    "managed-by:terraform",
  ]
}
