# Scaleway Container Registry for nvbes service images
resource "scaleway_registry_namespace" "main" {
  name        = "${var.name_prefix}-registry"
  project_id  = var.project_id
  region      = var.region
  is_public   = false
  description = "Container registry for ${var.environment} environment images"
}

# Scaleway Serverless Job for Account Worker Housekeeping
resource "scaleway_job_definition" "account_worker_housekeeping" {
  name                   = "${var.name_prefix}-account-worker-housekeeping"
  project_id             = var.project_id
  region                 = var.region
  cpu_limit              = 560
  memory_limit           = 1024
  local_storage_capacity = 1024

  image_uri = "${scaleway_registry_namespace.main.endpoint}/account-worker:latest"
  args      = ["run-housekeeping"]

  env = {
    NVBES_ENVIRONMENT = var.environment
  }

  cron {
    schedule = "0 * * * *" # Every hour
    timezone = "UTC"
  }
}

resource "scaleway_job_definition" "account_worker_audit_anchor" {
  count = var.enable_external_audit_archive ? 1 : 0

  name                   = "${var.name_prefix}-account-worker-audit-anchor"
  project_id             = var.project_id
  region                 = var.region
  cpu_limit              = 560
  memory_limit           = 1024
  local_storage_capacity = 1024

  image_uri = "${scaleway_registry_namespace.main.endpoint}/account-worker:latest"
  args      = ["run-audit-anchor"]

  env = {
    NVBES_ENVIRONMENT                = var.environment
    NVBES_AUDIT_ANCHOR_REGION        = var.region
    NVBES_AUDIT_ANCHOR_KMS_KEY_ID    = var.audit_anchor_kms_key_id
    NVBES_AUDIT_ANCHOR_BUCKET        = var.audit_archive_bucket_name
    NVBES_AUDIT_ANCHOR_S3_ENDPOINT   = "https://s3.${var.region}.scw.cloud"
    NVBES_AUDIT_ANCHOR_S3_ACCESS_KEY = var.audit_archive_writer_access_key
  }

  secret_reference {
    secret_id   = var.audit_anchor_kms_auth_token_secret_id
    environment = "NVBES_AUDIT_ANCHOR_KMS_AUTH_TOKEN"
  }

  secret_reference {
    secret_id   = var.audit_archive_writer_secret_id
    environment = "NVBES_AUDIT_ANCHOR_S3_SECRET_KEY"
  }

  secret_reference {
    secret_id   = var.account_database_url_secret_id
    environment = "NVBES_DATABASE_URL"
  }

  cron {
    schedule = "*/15 * * * *"
    timezone = "UTC"
  }
}

# Scaleway Serverless Job for Cloud Worker Maintenance (storage/geo cleanup)
resource "scaleway_job_definition" "cloud_worker_maintenance" {
  name                   = "${var.name_prefix}-cloud-worker-maintenance"
  project_id             = var.project_id
  region                 = var.region
  cpu_limit              = 560
  memory_limit           = 1024
  local_storage_capacity = 1024

  image_uri = "${scaleway_registry_namespace.main.endpoint}/cloud-worker:latest"
  args      = ["run-once"]

  env = {
    NVBES_ENVIRONMENT = var.environment
  }

  cron {
    schedule = "0 */6 * * *" # Every 6 hours
    timezone = "UTC"
  }
}

# Scaleway Serverless Job for Billing Worker Dunning & Reconciliation
resource "scaleway_job_definition" "billing_worker_jobs" {
  name                   = "${var.name_prefix}-billing-worker-jobs"
  project_id             = var.project_id
  region                 = var.region
  cpu_limit              = 560
  memory_limit           = 1024
  local_storage_capacity = 1024

  image_uri = "${scaleway_registry_namespace.main.endpoint}/billing-worker:latest"
  args      = ["run-billing-jobs"]

  env = {
    NVBES_ENVIRONMENT = var.environment
  }

  cron {
    schedule = "0 * * * *" # Every hour
    timezone = "UTC"
  }
}
