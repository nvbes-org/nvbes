resource "scaleway_sdb_sql_database" "identity" {
  name       = "${local.name_prefix}-identity"
  project_id = var.scaleway_project_id
  region     = var.scaleway_region
  min_cpu    = 0
  max_cpu    = 1

  lifecycle {
    prevent_destroy = true
  }
}

resource "scaleway_iam_application" "identity_database_runtime" {
  name        = "${local.name_prefix}-identity-database-runtime"
  description = "Identity runtime identity restricted to data access."
}

resource "scaleway_iam_policy" "identity_database_runtime" {
  name           = "${local.name_prefix}-identity-database-runtime"
  description    = "Can read and write Identity data without changing its schema."
  application_id = scaleway_iam_application.identity_database_runtime.id

  rule {
    project_ids          = [var.scaleway_project_id]
    permission_set_names = ["ServerlessSQLDatabaseDataReadWrite"]
  }
}

resource "scaleway_iam_api_key" "identity_database_runtime" {
  application_id     = scaleway_iam_application.identity_database_runtime.id
  default_project_id = var.scaleway_project_id
  description        = "Rotating data-only credential for Identity."
}

resource "scaleway_iam_application" "identity_database_migrator" {
  name        = "${local.name_prefix}-identity-database-migrator"
  description = "Identity migration identity allowed to update its schema."
}

resource "scaleway_iam_policy" "identity_database_migrator" {
  name           = "${local.name_prefix}-identity-database-migrator"
  description    = "Can apply Identity migrations without administering unrelated databases."
  application_id = scaleway_iam_application.identity_database_migrator.id

  rule {
    project_ids          = [var.scaleway_project_id]
    permission_set_names = ["ServerlessSQLDatabaseReadWrite"]
  }
}

resource "scaleway_iam_api_key" "identity_database_migrator" {
  application_id     = scaleway_iam_application.identity_database_migrator.id
  default_project_id = var.scaleway_project_id
  description        = "Rotating schema migration credential for Identity."
}

locals {
  identity_database_endpoint = split(
    "?",
    trimprefix(scaleway_sdb_sql_database.identity.endpoint, "postgres://"),
  )[0]
  identity_database_runtime_url = format(
    "postgres://%s:%s@%s?sslmode=verify-full",
    scaleway_iam_application.identity_database_runtime.id,
    scaleway_iam_api_key.identity_database_runtime.secret_key,
    local.identity_database_endpoint,
  )
  identity_database_migration_url = format(
    "postgres://%s:%s@%s?sslmode=verify-full",
    scaleway_iam_application.identity_database_migrator.id,
    scaleway_iam_api_key.identity_database_migrator.secret_key,
    local.identity_database_endpoint,
  )
}

resource "scaleway_secret" "identity_database_migration_url" {
  name        = "${local.name_prefix}-identity-database-migration-url"
  description = "DDL-capable database URL used only by the Identity migration job."
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  protected   = true
  tags        = local.tags
}

resource "scaleway_secret_version" "identity_database_migration_url" {
  secret_id   = scaleway_secret.identity_database_migration_url.id
  region      = var.scaleway_region
  data        = local.identity_database_migration_url
  description = "Terraform-managed Identity database migration credential."

  depends_on = [scaleway_iam_policy.identity_database_migrator]
}
