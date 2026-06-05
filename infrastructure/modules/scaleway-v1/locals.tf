locals {
  bucket_tags = {
    for tag in var.tags : replace(tag, ":", "_") => tag
  }
}
