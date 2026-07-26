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
  name        = "${var.name_prefix}-account-worker-housekeeping"
  project_id  = var.project_id
  region      = var.region
  cpu_limit   = 560
  memory_limit = 1024

  image_uri = "${scaleway_registry_namespace.main.endpoint}/account-worker:latest"
  command   = "run-housekeeping"

  env = {
    NVBES_ENVIRONMENT = var.environment
  }

  cron {
    schedule = "0 * * * *" # Every hour
    timezone = "UTC"
  }
# Scaleway Serverless Job for Cloud Worker Maintenance (storage/geo cleanup)
resource "scaleway_job_definition" "cloud_worker_maintenance" {
  name         = "${var.name_prefix}-cloud-worker-maintenance"
  project_id   = var.project_id
  region       = var.region
  cpu_limit    = 560
  memory_limit = 1024

  image_uri = "${scaleway_registry_namespace.main.endpoint}/cloud-worker:latest"
  command   = "run-once"

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
  name         = "${var.name_prefix}-billing-worker-jobs"
  project_id   = var.project_id
  region       = var.region
  cpu_limit    = 560
  memory_limit = 1024

  image_uri = "${scaleway_registry_namespace.main.endpoint}/billing-worker:latest"
  command   = "run-billing-jobs"

  env = {
    NVBES_ENVIRONMENT = var.environment
  }

  cron {
    schedule = "0 * * * *" # Every hour
    timezone = "UTC"
  }
}
