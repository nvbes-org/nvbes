resource "scaleway_vpc_private_network" "main" {
  name       = "${var.name_prefix}-pn"
  project_id = var.project_id
  region     = var.region

  ipv4_subnet {
    subnet = var.private_subnet
  }

  tags = var.tags
}

resource "scaleway_instance_security_group" "api" {
  name                    = "${var.name_prefix}-api-sg"
  project_id              = var.project_id
  inbound_default_policy  = "drop"
  outbound_default_policy = "accept"

  inbound_rule {
    action = "accept"
    port   = 80
  }

  inbound_rule {
    action = "accept"
    port   = 443
  }

  dynamic "inbound_rule" {
    for_each = var.ssh_allowed_ips

    content {
      action = "accept"
      port   = 22
      ip     = inbound_rule.value
    }
  }
}

resource "scaleway_instance_security_group" "worker" {
  name                    = "${var.name_prefix}-worker-sg"
  project_id              = var.project_id
  inbound_default_policy  = "drop"
  outbound_default_policy = "accept"

  dynamic "inbound_rule" {
    for_each = var.ssh_allowed_ips

    content {
      action = "accept"
      port   = 22
      ip     = inbound_rule.value
    }
  }
}
