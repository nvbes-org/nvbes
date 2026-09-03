output "identity_endpoint" {
  description = "Public endpoint exposing only Identity health and protected metrics."
  value       = scaleway_container.identity.public_endpoint
}

output "identity_registry_endpoint" {
  description = "Private Scaleway Registry receiving verified Identity images."
  value       = scaleway_registry_namespace.identity.endpoint
}

output "identity_database_id" {
  description = "Dedicated Serverless SQL database used by Identity."
  value       = scaleway_sdb_sql_database.identity.id
}

output "identity_database_runtime_url" {
  description = "Sensitive data-only database URL consumed by runtime validation."
  sensitive   = true
  value       = local.identity_database_runtime_url
}

output "identity_database_migration_job_id" {
  description = "Job definition applying Identity SQLx migrations."
  value       = scaleway_job_definition.identity_database_migration.id
}

output "identity_synthetic_auth_job_id" {
  description = "Job definition proving Identity authentication and recovery without public routes."
  value       = scaleway_job_definition.identity_synthetic_auth.id
}

output "identity_error_reporting_smoke_job_id" {
  description = "Job definition proving Identity Sentry event delivery."
  value       = scaleway_job_definition.identity_error_reporting_smoke.id
}

output "identity_synthetic_token_job_id" {
  description = "Job definition proving audience-bound Account access-token issuance."
  value       = scaleway_job_definition.identity_synthetic_token.id
}

output "identity_synthetic_mfa_job_id" {
  description = "Job definition proving encrypted TOTP step-up in production."
  value       = scaleway_job_definition.identity_synthetic_mfa.id
}

output "identity_synthetic_invitation_job_id" {
  description = "Job definition proving invitation-only account creation in production."
  value       = scaleway_job_definition.identity_synthetic_invitation.id
}

output "identity_token_issuer" {
  description = "Public issuer passed only to the explicit private token proof job."
  value       = var.identity_token_issuer
}

output "identity_token_key_id" {
  description = "Non-secret signing key identifier passed only to the explicit private token proof job."
  value       = var.identity_token_key_id
}

output "identity_token_audiences" {
  description = "Fixed V1 audiences passed only to the explicit private token proof job."
  value       = var.identity_token_audiences
}

output "identity_mfa_key_version" {
  description = "Active non-secret MFA key version passed only to the explicit private MFA proof job."
  value       = var.identity_mfa_key_version
}
