resource "scaleway_sdb_sql_database" "account" {
  name       = "${local.name_prefix}-account"
  project_id = var.scaleway_project_id
  region     = var.scaleway_region
  min_cpu    = 0
  max_cpu    = 1

  lifecycle {
    prevent_destroy = true
  }
}

resource "scaleway_iam_application" "account_database_runtime" {
  name        = "${local.name_prefix}-account-database-runtime"
  description = "Account runtime identity restricted to data access."
}

resource "scaleway_iam_policy" "account_database_runtime" {
  name           = "${local.name_prefix}-account-database-runtime"
  description    = "Can read and write Account data without changing its schema."
  application_id = scaleway_iam_application.account_database_runtime.id

  rule {
    project_ids          = [var.scaleway_project_id]
    permission_set_names = ["ServerlessSQLDatabaseDataReadWrite"]
  }
}

resource "scaleway_iam_api_key" "account_database_runtime" {
  application_id     = scaleway_iam_application.account_database_runtime.id
  default_project_id = var.scaleway_project_id
  description        = "Rotating data-only credential for Account."
}

resource "scaleway_iam_application" "account_database_migrator" {
  name        = "${local.name_prefix}-account-database-migrator"
  description = "Account migration identity allowed to update its schema."
}

resource "scaleway_iam_policy" "account_database_migrator" {
  name           = "${local.name_prefix}-account-database-migrator"
  description    = "Can apply Account migrations without administering unrelated databases."
  application_id = scaleway_iam_application.account_database_migrator.id

  rule {
    project_ids          = [var.scaleway_project_id]
    permission_set_names = ["ServerlessSQLDatabaseReadWrite"]
  }
}

resource "scaleway_iam_api_key" "account_database_migrator" {
  application_id     = scaleway_iam_application.account_database_migrator.id
  default_project_id = var.scaleway_project_id
  description        = "Rotating schema migration credential for Account."
}

locals {
  account_database_endpoint = split(
    "?",
    trimprefix(scaleway_sdb_sql_database.account.endpoint, "postgres://"),
  )[0]
  account_database_runtime_url = format(
    "postgres://%s:%s@%s?sslmode=verify-full",
    scaleway_iam_application.account_database_runtime.id,
    scaleway_iam_api_key.account_database_runtime.secret_key,
    local.account_database_endpoint,
  )
  account_database_migration_url = format(
    "postgres://%s:%s@%s?sslmode=verify-full",
    scaleway_iam_application.account_database_migrator.id,
    scaleway_iam_api_key.account_database_migrator.secret_key,
    local.account_database_endpoint,
  )
}

resource "scaleway_secret" "account_database_migration_url" {
  name        = "${local.name_prefix}-account-database-migration-url"
  description = "DDL-capable database URL used only by the Account migration job."
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  protected   = true
  tags        = local.tags
}

resource "scaleway_secret_version" "account_database_migration_url" {
  secret_id   = scaleway_secret.account_database_migration_url.id
  region      = var.scaleway_region
  data        = local.account_database_migration_url
  description = "Terraform-managed Account database migration credential."

  depends_on = [scaleway_iam_policy.account_database_migrator]
}

resource "scaleway_secret" "account_database_runtime_url" {
  name        = "${local.name_prefix}-account-database-runtime-url"
  description = "Data-only database URL used by Account runtime jobs."
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  protected   = true
  tags        = local.tags
}

resource "scaleway_secret_version" "account_database_runtime_url" {
  secret_id   = scaleway_secret.account_database_runtime_url.id
  region      = var.scaleway_region
  data        = local.account_database_runtime_url
  description = "Terraform-managed Account database runtime credential."

  depends_on = [scaleway_iam_policy.account_database_runtime]
}
