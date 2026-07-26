resource "scaleway_instance_ip" "api" {
  project_id = var.project_id
  zone       = var.zone
}

resource "scaleway_instance_ip" "worker" {
  count      = var.enable_legacy_vm_worker ? 1 : 0
  project_id = var.project_id
  zone       = var.zone
}

resource "scaleway_instance_server" "api" {
  name              = "${var.name_prefix}-api"
  project_id        = var.project_id
  zone              = var.zone
  type              = var.api_instance_type
  image             = var.instance_image
  ip_id             = scaleway_instance_ip.api.id
  security_group_id = scaleway_instance_security_group.api.id
  tags              = var.tags

  root_volume {
    size_in_gb = 20
  }
}

resource "scaleway_instance_server" "worker" {
  count             = var.enable_legacy_vm_worker ? 1 : 0
  name              = "${var.name_prefix}-worker"
  project_id        = var.project_id
  zone              = var.zone
  type              = var.worker_instance_type
  image             = var.instance_image
  ip_id             = scaleway_instance_ip.worker[0].id
  security_group_id = scaleway_instance_security_group.worker.id
  tags              = var.tags

  root_volume {
    size_in_gb = 20
  }
}

resource "scaleway_instance_private_nic" "api" {
  server_id          = scaleway_instance_server.api.id
  private_network_id = scaleway_vpc_private_network.main.id
}

resource "scaleway_instance_private_nic" "worker" {
  count              = var.enable_legacy_vm_worker ? 1 : 0
  server_id          = scaleway_instance_server.worker[0].id
  private_network_id = scaleway_vpc_private_network.main.id
}
