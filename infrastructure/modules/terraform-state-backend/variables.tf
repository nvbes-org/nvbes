variable "project_id" {
  description = "Dedicated Scaleway project owning Terraform state resources."
  type        = string
}

variable "region" {
  description = "Scaleway Object Storage region."
  type        = string
}

variable "environment" {
  description = "Environment encoded in every state object prefix."
  type        = string
}

variable "bucket_name" {
  description = "Globally unique shared Terraform state bucket name."
  type        = string
}

variable "state_stacks" {
  description = "Stacks receiving isolated credentials and state prefixes."
  type        = set(string)

  validation {
    condition = (
      contains(var.state_stacks, "bootstrap") &&
      alltrue([
        for stack in var.state_stacks : can(regex("^[a-z][a-z0-9-]*$", stack))
      ])
    )
    error_message = "state_stacks must contain bootstrap and use lowercase kebab-case names."
  }
}

variable "external_state_application_ids" {
  description = "Pre-bootstrapped state application IDs receiving isolated prefixes without duplicate IAM resources."
  type        = map(string)
  default     = {}

  validation {
    condition = alltrue([
      for stack, application_id in var.external_state_application_ids :
      can(regex("^[a-z][a-z0-9-]*$", stack)) &&
      can(regex("^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$", application_id)) &&
      !contains(var.state_stacks, stack)
    ])
    error_message = "external state stacks must be unique lowercase kebab-case names mapped to Scaleway application UUIDs."
  }
}

variable "tags" {
  description = "Tags applied to the state bucket."
  type        = list(string)
}
