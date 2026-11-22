resource "scaleway_sdb_sql_database" "nvbes" {
  name       = "${local.name_prefix}-nvbes"
  project_id = var.scaleway_project_id
  region     = var.scaleway_region
  min_cpu    = 0
  max_cpu    = 1

  lifecycle {
    prevent_destroy = true
  }
}

resource "scaleway_iam_application" "api_database_runtime" {
  name        = "${local.name_prefix}-api-database-runtime"
  description = "API runtime identity restricted to data access."
}

resource "scaleway_iam_policy" "api_database_runtime" {
  name           = "${local.name_prefix}-api-database-runtime"
  description    = "Can read and write API data without changing its schema."
  application_id = scaleway_iam_application.api_database_runtime.id

  rule {
    project_ids          = [var.scaleway_project_id]
    permission_set_names = ["ServerlessSQLDatabaseDataReadWrite"]
  }
}

resource "scaleway_iam_api_key" "api_database_runtime" {
  application_id     = scaleway_iam_application.api_database_runtime.id
  default_project_id = var.scaleway_project_id
  description        = "Rotating data-only credential for API."
}

resource "scaleway_iam_application" "api_database_migrator" {
  name        = "${local.name_prefix}-api-database-migrator"
  description = "API migration identity allowed to update database schema."
}

resource "scaleway_iam_policy" "api_database_migrator" {
  name           = "${local.name_prefix}-api-database-migrator"
  description    = "Can apply database migrations without administering unrelated databases."
  application_id = scaleway_iam_application.api_database_migrator.id

  rule {
    project_ids          = [var.scaleway_project_id]
    permission_set_names = ["ServerlessSQLDatabaseReadWrite"]
  }
}

resource "scaleway_iam_api_key" "api_database_migrator" {
  application_id     = scaleway_iam_application.api_database_migrator.id
  default_project_id = var.scaleway_project_id
  description        = "Rotating schema migration credential for API."
}

locals {
  api_database_endpoint = split(
    "?",
    trimprefix(scaleway_sdb_sql_database.nvbes.endpoint, "postgres://"),
  )[0]
  api_database_runtime_url = format(
    "postgres://%s:%s@%s?sslmode=verify-full",
    scaleway_iam_application.api_database_runtime.id,
    scaleway_iam_api_key.api_database_runtime.secret_key,
    local.api_database_endpoint,
  )
  api_database_migration_url = format(
    "postgres://%s:%s@%s?sslmode=verify-full",
    scaleway_iam_application.api_database_migrator.id,
    scaleway_iam_api_key.api_database_migrator.secret_key,
    local.api_database_endpoint,
  )
}

resource "scaleway_secret" "api_database_migration_url" {
  name        = "${local.name_prefix}-api-database-migration-url"
  description = "DDL-capable database URL used for schema migrations."
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  protected   = true
  tags        = local.tags
}

resource "scaleway_secret_version" "api_database_migration_url" {
  secret_id   = scaleway_secret.api_database_migration_url.id
  region      = var.scaleway_region
  data        = local.api_database_migration_url
  description = "Terraform-managed API database migration credential."

  depends_on = [scaleway_iam_policy.api_database_migrator]
}

resource "scaleway_secret" "api_database_runtime_url" {
  name        = "${local.name_prefix}-api-database-runtime-url"
  description = "Data-only database URL used by API runtime."
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  protected   = true
  tags        = local.tags
}

resource "scaleway_secret_version" "api_database_runtime_url" {
  secret_id   = scaleway_secret.api_database_runtime_url.id
  region      = var.scaleway_region
  data        = local.api_database_runtime_url
  description = "Terraform-managed API database runtime credential."

  depends_on = [scaleway_iam_policy.api_database_runtime]
}
