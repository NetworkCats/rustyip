resource "vultr_instance" "rustyip" {
  plan              = var.plan
  region            = var.region
  os_id             = var.os_id
  label             = var.instance_label
  hostname          = var.instance_label
  enable_ipv6       = false
  activation_email  = false
  backups           = "disabled"
  ddos_protection   = false
  firewall_group_id = vultr_firewall_group.rustyip.id
  ssh_key_ids       = [vultr_ssh_key.deploy.id]

  tags = [var.instance_label]
}
