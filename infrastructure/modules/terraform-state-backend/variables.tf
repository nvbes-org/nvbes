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

variable "tags" {
  description = "Tags applied to the state bucket."
  type        = list(string)
}
