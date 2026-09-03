resource "scaleway_sdb_sql_database" "billing" {
  name       = "${local.name_prefix}-billing"
  project_id = var.scaleway_project_id
  region     = var.scaleway_region
  min_cpu    = 0
  max_cpu    = 1

  lifecycle {
    prevent_destroy = true
  }
}

resource "scaleway_iam_application" "billing_database_runtime" {
  name        = "${local.name_prefix}-billing-database-runtime"
  description = "Billing runtime identity restricted to data access."
}

resource "scaleway_iam_policy" "billing_database_runtime" {
  name           = "${local.name_prefix}-billing-database-runtime"
  description    = "Can read and write Billing data without changing its schema."
  application_id = scaleway_iam_application.billing_database_runtime.id

  rule {
    project_ids          = [var.scaleway_project_id]
    permission_set_names = ["ServerlessSQLDatabaseDataReadWrite"]
  }
}

resource "scaleway_iam_api_key" "billing_database_runtime" {
  application_id     = scaleway_iam_application.billing_database_runtime.id
  default_project_id = var.scaleway_project_id
  description        = "Rotating data-only credential for Billing."
}

resource "scaleway_iam_application" "billing_database_migrator" {
  name        = "${local.name_prefix}-billing-database-migrator"
  description = "Billing migration identity allowed to update its schema."
}

resource "scaleway_iam_policy" "billing_database_migrator" {
  name           = "${local.name_prefix}-billing-database-migrator"
  description    = "Can apply Billing migrations without administering unrelated databases."
  application_id = scaleway_iam_application.billing_database_migrator.id

  rule {
    project_ids          = [var.scaleway_project_id]
    permission_set_names = ["ServerlessSQLDatabaseReadWrite"]
  }
}

resource "scaleway_iam_api_key" "billing_database_migrator" {
  application_id     = scaleway_iam_application.billing_database_migrator.id
  default_project_id = var.scaleway_project_id
  description        = "Rotating schema migration credential for Billing."
}

locals {
  billing_database_endpoint = split(
    "?",
    trimprefix(scaleway_sdb_sql_database.billing.endpoint, "postgres://"),
  )[0]
  billing_database_runtime_url = format(
    "postgres://%s:%s@%s?sslmode=verify-full",
    scaleway_iam_application.billing_database_runtime.id,
    scaleway_iam_api_key.billing_database_runtime.secret_key,
    local.billing_database_endpoint,
  )
  billing_database_migration_url = format(
    "postgres://%s:%s@%s?sslmode=verify-full",
    scaleway_iam_application.billing_database_migrator.id,
    scaleway_iam_api_key.billing_database_migrator.secret_key,
    local.billing_database_endpoint,
  )
}

resource "scaleway_secret" "billing_database_migration_url" {
  name        = "${local.name_prefix}-billing-database-migration-url"
  description = "DDL-capable database URL used only by the Billing migration job."
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  protected   = true
  tags        = local.tags
}

resource "scaleway_secret_version" "billing_database_migration_url" {
  secret_id   = scaleway_secret.billing_database_migration_url.id
  region      = var.scaleway_region
  data        = local.billing_database_migration_url
  description = "Terraform-managed Billing database migration credential."

  depends_on = [scaleway_iam_policy.billing_database_migrator]
}

resource "scaleway_job_definition" "billing_database_migration" {
  name                   = "${local.name_prefix}-billing-database-migration"
  description            = "Explicit Billing schema migration run before runtime deployment."
  project_id             = var.scaleway_project_id
  region                 = var.scaleway_region
  cpu_limit              = 560
  memory_limit           = 1024
  local_storage_capacity = 1024
  image_uri              = var.billing_image
  args                   = ["migrate"]

  secret_reference {
    secret_id   = scaleway_secret.billing_database_migration_url.id
    environment = "NVBES_BILLING_DATABASE_URL"
  }

  depends_on = [
    scaleway_iam_policy.billing_database_migrator,
    scaleway_registry_namespace.billing,
    scaleway_secret_version.billing_database_migration_url,
  ]
}
