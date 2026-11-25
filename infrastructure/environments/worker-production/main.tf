locals {
  environment = "production"
  name_prefix = "nvbes-prod"
  tags = [
    "nvbes",
    "component:worker",
    "environment:production",
    "managed-by:terraform",
  ]
}
