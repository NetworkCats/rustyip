resource "vultr_ssh_key" "deploy" {
  name    = "${var.instance_label}-deploy"
  ssh_key = var.ssh_public_key
}
