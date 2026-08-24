resource "scaleway_sdb_sql_database" "email" {
  name       = "${local.name_prefix}-email"
  project_id = var.scaleway_project_id
  region     = var.scaleway_region
  min_cpu    = 0
  max_cpu    = 1

  lifecycle {
    prevent_destroy = true
  }
}

resource "scaleway_iam_application" "email_database_runtime" {
  name        = "${local.name_prefix}-email-database-runtime"
  description = "Email runtime identity restricted to ledger data access."
}

resource "scaleway_iam_policy" "email_database_runtime" {
  name           = "${local.name_prefix}-email-database-runtime"
  description    = "Can read and write email ledger rows without changing its schema."
  application_id = scaleway_iam_application.email_database_runtime.id

  rule {
    project_ids          = [var.scaleway_project_id]
    permission_set_names = ["ServerlessSQLDatabaseDataReadWrite"]
  }
}

resource "scaleway_iam_api_key" "email_database_runtime" {
  application_id     = scaleway_iam_application.email_database_runtime.id
  default_project_id = var.scaleway_project_id
  description        = "Rotating data-only credential for the email runtime."
}

resource "scaleway_iam_application" "email_database_migrator" {
  name        = "${local.name_prefix}-email-database-migrator"
  description = "Email migration identity allowed to update the ledger schema."
}

resource "scaleway_iam_policy" "email_database_migrator" {
  name           = "${local.name_prefix}-email-database-migrator"
  description    = "Can apply email ledger migrations without administering the database."
  application_id = scaleway_iam_application.email_database_migrator.id

  rule {
    project_ids          = [var.scaleway_project_id]
    permission_set_names = ["ServerlessSQLDatabaseReadWrite"]
  }
}

resource "scaleway_iam_api_key" "email_database_migrator" {
  application_id     = scaleway_iam_application.email_database_migrator.id
  default_project_id = var.scaleway_project_id
  description        = "Rotating schema migration credential for email-worker."
}

locals {
  email_database_endpoint = split(
    "?",
    trimprefix(scaleway_sdb_sql_database.email.endpoint, "postgres://"),
  )[0]
  email_database_runtime_url = format(
    "postgres://%s:%s@%s?sslmode=verify-full",
    scaleway_iam_application.email_database_runtime.id,
    scaleway_iam_api_key.email_database_runtime.secret_key,
    local.email_database_endpoint,
  )
  email_database_migration_url = format(
    "postgres://%s:%s@%s?sslmode=verify-full",
    scaleway_iam_application.email_database_migrator.id,
    scaleway_iam_api_key.email_database_migrator.secret_key,
    local.email_database_endpoint,
  )
}

resource "scaleway_secret" "email_database_migration_url" {
  name        = "${local.name_prefix}-email-database-migration-url"
  description = "DDL-capable Serverless SQL URL used only by the email migration job."
  project_id  = var.scaleway_project_id
  region      = var.scaleway_region
  protected   = true
  tags        = local.tags
}

resource "scaleway_secret_version" "email_database_migration_url" {
  secret_id   = scaleway_secret.email_database_migration_url.id
  region      = var.scaleway_region
  data        = local.email_database_migration_url
  description = "Terraform-managed email database migration credential."

  depends_on = [scaleway_iam_policy.email_database_migrator]
}

resource "scaleway_job_definition" "email_database_migration" {
  name                   = "${local.name_prefix}-email-database-migration"
  description            = "Explicit email-worker schema migration run before runtime deployment."
  project_id             = var.scaleway_project_id
  region                 = var.scaleway_region
  cpu_limit              = 280
  memory_limit           = 512
  local_storage_capacity = 1024
  image_uri              = var.email_worker_image
  args                   = ["migrate"]

  secret_reference {
    secret_id   = scaleway_secret.email_database_migration_url.id
    environment = "NVBES_EMAIL_DATABASE_URL"
  }

  depends_on = [
    scaleway_iam_policy.email_database_migrator,
    scaleway_registry_namespace.email_worker,
    scaleway_secret_version.email_database_migration_url,
  ]
}
