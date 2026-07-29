variable "project_id" {
  description = "Security-account project that owns the immutable archive."
  type        = string
}

variable "region" {
  type = string
}

variable "name_prefix" {
  type = string
}

variable "bucket_name" {
  type = string
}

variable "retention_years" {
  type    = number
  default = 7

  validation {
    condition     = var.retention_years >= 1
    error_message = "Audit evidence retention must be at least one year."
  }
}

variable "tags" {
  type    = list(string)
  default = []
}
