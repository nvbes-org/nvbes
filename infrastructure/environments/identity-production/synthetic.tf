resource "scaleway_secret" "identity_mfa_encryption_key" {
  name        = "${local.name_prefix}-identity-mfa-encryption-key"
  description = "MFA encryption key used only by the Identity runtime and synthetic MFA proof."
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  protected   = true
  tags        = local.tags
}

resource "scaleway_secret_version" "identity_mfa_encryption_key" {
  secret_id   = scaleway_secret.identity_mfa_encryption_key.id
  region      = var.scaleway_region
  data        = var.identity_mfa_encryption_key
  description = "Terraform-managed active Identity MFA encryption key."
}

resource "scaleway_secret" "identity_token_private_key" {
  name        = "${local.name_prefix}-identity-token-private-key"
  description = "RS256 private signing key used only by Identity token issuance."
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  protected   = true
  tags        = local.tags
}

resource "scaleway_secret_version" "identity_token_private_key" {
  secret_id   = scaleway_secret.identity_token_private_key.id
  region      = var.scaleway_region
  data        = var.identity_token_private_key_pem
  description = "Terraform-managed private Identity token signing key."
}

resource "scaleway_secret" "identity_token_public_key" {
  name        = "${local.name_prefix}-identity-token-public-key"
  description = "RS256 public verification key used by the Identity token proof job."
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  protected   = true
  tags        = local.tags
}

resource "scaleway_secret_version" "identity_token_public_key" {
  secret_id   = scaleway_secret.identity_token_public_key.id
  region      = var.scaleway_region
  data        = var.identity_token_public_key_pem
  description = "Terraform-managed public Identity token verification key."
}

resource "scaleway_job_definition" "identity_synthetic_token" {
  name                   = "${local.name_prefix}-identity-synthetic-token"
  description            = "Private proof of session-derived Account token claims and revocation."
  project_id             = var.scaleway_project_id
  region                 = var.scaleway_region
  cpu_limit              = 560
  memory_limit           = 1024
  local_storage_capacity = 1024
  image_uri              = var.identity_image
  args                   = ["synthetic-token-smoke"]

  secret_reference {
    secret_id   = scaleway_secret.identity_database_runtime_url.id
    environment = "NVBES_IDENTITY_DATABASE_URL"
  }

  secret_reference {
    secret_id   = scaleway_secret.identity_synthetic_password.id
    environment = "NVBES_IDENTITY_SYNTHETIC_PASSWORD"
  }

  secret_reference {
    secret_id   = scaleway_secret.identity_token_private_key.id
    environment = "NVBES_IDENTITY_TOKEN_PRIVATE_KEY_PEM"
  }

  secret_reference {
    secret_id   = scaleway_secret.identity_token_public_key.id
    environment = "NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM"
  }

  depends_on = [
    scaleway_registry_namespace.identity,
    scaleway_secret_version.identity_database_runtime_url,
    scaleway_secret_version.identity_synthetic_password,
    scaleway_secret_version.identity_token_private_key,
    scaleway_secret_version.identity_token_public_key,
  ]
}

resource "scaleway_job_definition" "identity_synthetic_mfa" {
  name                   = "${local.name_prefix}-identity-synthetic-mfa"
  description            = "Private proof of encrypted TOTP enrollment, step-up and replay rejection."
  project_id             = var.scaleway_project_id
  region                 = var.scaleway_region
  cpu_limit              = 560
  memory_limit           = 1024
  local_storage_capacity = 1024
  image_uri              = var.identity_image
  args                   = ["synthetic-mfa-smoke"]

  secret_reference {
    secret_id   = scaleway_secret.identity_database_runtime_url.id
    environment = "NVBES_IDENTITY_DATABASE_URL"
  }

  secret_reference {
    secret_id   = scaleway_secret.identity_synthetic_password.id
    environment = "NVBES_IDENTITY_SYNTHETIC_PASSWORD"
  }

  secret_reference {
    secret_id   = scaleway_secret.identity_mfa_encryption_key.id
    environment = "NVBES_IDENTITY_MFA_ENCRYPTION_KEY"
  }

  depends_on = [
    scaleway_registry_namespace.identity,
    scaleway_secret_version.identity_database_runtime_url,
    scaleway_secret_version.identity_synthetic_password,
    scaleway_secret_version.identity_mfa_encryption_key,
  ]
}

resource "scaleway_job_definition" "identity_synthetic_invitation" {
  name                   = "${local.name_prefix}-identity-synthetic-invitation"
  description            = "Private proof of invitation-only one-time account creation."
  project_id             = var.scaleway_project_id
  region                 = var.scaleway_region
  cpu_limit              = 560
  memory_limit           = 1024
  local_storage_capacity = 1024
  image_uri              = var.identity_image
  args                   = ["synthetic-invitation-smoke"]

  secret_reference {
    secret_id   = scaleway_secret.identity_database_runtime_url.id
    environment = "NVBES_IDENTITY_DATABASE_URL"
  }

  secret_reference {
    secret_id   = scaleway_secret.identity_synthetic_password.id
    environment = "NVBES_IDENTITY_SYNTHETIC_PASSWORD"
  }

  depends_on = [
    scaleway_registry_namespace.identity,
    scaleway_secret_version.identity_database_runtime_url,
    scaleway_secret_version.identity_synthetic_password,
  ]
}
