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

# Allow HTTP from Cloudflare only (IPv4)
resource "vultr_firewall_rule" "http_cloudflare_v4" {
  firewall_group_id = vultr_firewall_group.rustyip.id
  protocol          = "tcp"
  ip_type           = "v4"
  subnet            = "0.0.0.0"
  subnet_size       = 0
  port              = "80"
  source            = "cloudflare"
  notes             = "Allow HTTP from Cloudflare IPv4"
}

# Allow HTTPS from Cloudflare only (IPv4)
resource "vultr_firewall_rule" "https_cloudflare_v4" {
  firewall_group_id = vultr_firewall_group.rustyip.id
  protocol          = "tcp"
  ip_type           = "v4"
  subnet            = "0.0.0.0"
  subnet_size       = 0
  port              = "443"
  source            = "cloudflare"
  notes             = "Allow HTTPS from Cloudflare IPv4"
}
