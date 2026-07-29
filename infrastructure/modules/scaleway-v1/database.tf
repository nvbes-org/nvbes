resource "scaleway_rdb_instance" "postgres" {
  name                      = "${var.name_prefix}-postgres"
  project_id                = var.project_id
  region                    = var.region
  node_type                 = var.rdb_node_type
  engine                    = var.postgres_engine
  user_name                 = var.postgres_user
  password                  = var.postgres_password
  encryption_at_rest        = true
  disable_backup            = false
  backup_schedule_frequency = 24
  backup_schedule_retention = var.postgres_backup_retention_days
  # Keep provider-managed logical backups outside the database region so a
  # regional control-plane incident does not share the primary failure domain.
  backup_same_region = false
  volume_type        = "bssd"
  volume_size_in_gb  = 10

  private_network {
    pn_id       = scaleway_vpc_private_network.main.id
    enable_ipam = true
  }

  tags = var.tags
}
