output "instance_id" {
  description = "Vultr instance ID"
  value       = vultr_instance.rustyip.id
}

output "instance_ip" {
  description = "VPS public IPv4 address"
  value       = vultr_instance.rustyip.main_ip
  sensitive   = true
}
