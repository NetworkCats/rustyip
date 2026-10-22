resource "vultr_firewall_group" "rustyip" {
  description = "${var.instance_label} firewall"
}

# Allow SSH from anywhere (IPv4)
resource "vultr_firewall_rule" "ssh_v4" {
  firewall_group_id = vultr_firewall_group.rustyip.id
  protocol          = "tcp"
  ip_type           = "v4"
  subnet            = "0.0.0.0"
  subnet_size       = 0
  port              = "22"
  notes             = "Allow SSH from anywhere"
}

# Allow HTTPS from anywhere (IPv4). The host restricts it to Cloudflare's
# ranges with UFW (ansible firewall role), which keeps the list current.
resource "vultr_firewall_rule" "https_v4" {
  firewall_group_id = vultr_firewall_group.rustyip.id
  protocol          = "tcp"
  ip_type           = "v4"
  subnet            = "0.0.0.0"
  subnet_size       = 0
  port              = "443"
  notes             = "Allow HTTPS; host UFW limits it to Cloudflare"
}
