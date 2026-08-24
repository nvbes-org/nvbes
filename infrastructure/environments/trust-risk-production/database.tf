resource "scaleway_sdb_sql_database" "trust_risk" {
  name       = "${local.name_prefix}-trust-risk"
  project_id = var.scaleway_project_id
  region     = var.scaleway_region
  min_cpu    = 0
  max_cpu    = 2

  lifecycle {
    prevent_destroy = true
  }
}

resource "scaleway_iam_application" "trust_risk_database_runtime" {
  name        = "${local.name_prefix}-trust-risk-database-runtime"
  description = "Trust/Risk runtime identity restricted to data access."
}

resource "scaleway_iam_policy" "trust_risk_database_runtime" {
  name           = "${local.name_prefix}-trust-risk-database-runtime"
  description    = "Can read and write Trust/Risk data without changing its schema."
  application_id = scaleway_iam_application.trust_risk_database_runtime.id

  rule {
    project_ids          = [var.scaleway_project_id]
    permission_set_names = ["ServerlessSQLDatabaseDataReadWrite"]
  }
}

resource "scaleway_iam_api_key" "trust_risk_database_runtime" {
  application_id     = scaleway_iam_application.trust_risk_database_runtime.id
  default_project_id = var.scaleway_project_id
  description        = "Rotating data-only credential for Trust/Risk."
}

resource "scaleway_iam_application" "trust_risk_database_migrator" {
  name        = "${local.name_prefix}-trust-risk-database-migrator"
  description = "Trust/Risk migration identity allowed to update its schema."
}

resource "scaleway_iam_policy" "trust_risk_database_migrator" {
  name           = "${local.name_prefix}-trust-risk-database-migrator"
  description    = "Can apply Trust/Risk migrations without administering unrelated databases."
  application_id = scaleway_iam_application.trust_risk_database_migrator.id

  rule {
    project_ids          = [var.scaleway_project_id]
    permission_set_names = ["ServerlessSQLDatabaseReadWrite"]
  }
}

resource "scaleway_iam_api_key" "trust_risk_database_migrator" {
  application_id     = scaleway_iam_application.trust_risk_database_migrator.id
  default_project_id = var.scaleway_project_id
  description        = "Rotating schema migration credential for Trust/Risk."
}

locals {
  trust_risk_database_endpoint = split(
    "?",
    trimprefix(scaleway_sdb_sql_database.trust_risk.endpoint, "postgres://"),
  )[0]
  trust_risk_database_runtime_url = format(
    "postgres://%s:%s@%s?sslmode=verify-full",
    scaleway_iam_application.trust_risk_database_runtime.id,
    scaleway_iam_api_key.trust_risk_database_runtime.secret_key,
    local.trust_risk_database_endpoint,
  )
  trust_risk_database_migration_url = format(
    "postgres://%s:%s@%s?sslmode=verify-full",
    scaleway_iam_application.trust_risk_database_migrator.id,
    scaleway_iam_api_key.trust_risk_database_migrator.secret_key,
    local.trust_risk_database_endpoint,
  )
}

resource "scaleway_secret" "trust_risk_database_migration_url" {
  name        = "${local.name_prefix}-trust-risk-database-migration-url"
  description = "DDL-capable database URL used only by the Trust/Risk migration job."
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  protected   = true
  tags        = local.tags
}

resource "scaleway_secret_version" "trust_risk_database_migration_url" {
  secret_id   = scaleway_secret.trust_risk_database_migration_url.id
  region      = var.scaleway_region
  data        = local.trust_risk_database_migration_url
  description = "Terraform-managed Trust/Risk database migration credential."

  depends_on = [scaleway_iam_policy.trust_risk_database_migrator]
}

resource "scaleway_job_definition" "trust_risk_database_migration" {
  name                   = "${local.name_prefix}-trust-risk-database-migration"
  description            = "Explicit Trust/Risk schema migration run before runtime deployment."
  project_id             = var.scaleway_project_id
  region                 = var.scaleway_region
  cpu_limit              = 560
  memory_limit           = 1024
  local_storage_capacity = 1024
  image_uri              = var.trust_risk_image
  args                   = ["migrate"]

  secret_reference {
    secret_id   = scaleway_secret.trust_risk_database_migration_url.id
    environment = "NVBES_TRUST_RISK_DATABASE_URL"
  }

  depends_on = [
    scaleway_iam_policy.trust_risk_database_migrator,
    scaleway_registry_namespace.trust_risk,
    scaleway_secret_version.trust_risk_database_migration_url,
  ]
}
