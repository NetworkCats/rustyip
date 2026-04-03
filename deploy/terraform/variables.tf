variable "vultr_api_key" {
  description = "Vultr API key"
  type        = string
  sensitive   = true
}

variable "ssh_public_key" {
  description = "SSH public key for VPS access"
  type        = string
}

variable "region" {
  description = "Vultr region ID"
  type        = string
  default     = "dfw"
}

variable "plan" {
  description = "Vultr plan ID (Regular Performance 1 vCPU / 1 GB)"
  type        = string
  default     = "vc2-1c-1gb"
}

variable "os_id" {
  description = "Vultr OS ID (Debian 13 Trixie x64)"
  type        = number
  default     = 2625
}

variable "instance_label" {
  description = "Label for the VPS instance"
  type        = string
  default     = "rustyip"
}
