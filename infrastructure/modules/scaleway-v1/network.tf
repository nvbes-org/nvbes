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
  outbound_default_policy = "drop"

  dynamic "inbound_rule" {
    for_each = var.edge_allowed_ipv4_cidrs

    content {
      action = "accept"
      port   = 443
      ip     = inbound_rule.value
    }
  }

  dynamic "inbound_rule" {
    for_each = var.enable_jit_ssh ? var.ssh_allowed_ips : []

    content {
      action = "accept"
      port   = 22
      ip     = inbound_rule.value
    }
  }

  dynamic "outbound_rule" {
    for_each = var.egress_https_allowed_cidrs

    content {
      action = "accept"
      port   = 443
      ip     = outbound_rule.value
    }
  }

  outbound_rule {
    action   = "accept"
    port     = 53
    protocol = "UDP"
  }

  outbound_rule {
    action = "accept"
    port   = 5432
    ip     = var.private_subnet
  }
}

resource "scaleway_instance_security_group" "worker" {
  name                    = "${var.name_prefix}-worker-sg"
  project_id              = var.project_id
  inbound_default_policy  = "drop"
  outbound_default_policy = "drop"

  dynamic "inbound_rule" {
    for_each = var.enable_jit_ssh ? var.ssh_allowed_ips : []

    content {
      action = "accept"
      port   = 22
      ip     = inbound_rule.value
    }
  }

  dynamic "outbound_rule" {
    for_each = var.egress_https_allowed_cidrs

    content {
      action = "accept"
      port   = 443
      ip     = outbound_rule.value
    }
  }

  outbound_rule {
    action   = "accept"
    port     = 53
    protocol = "UDP"
  }

  outbound_rule {
    action = "accept"
    port   = 5432
    ip     = var.private_subnet
  }
}
