terraform {
  required_version = ">= 1.5"

  required_providers {
    vultr = {
      source  = "vultr/vultr"
      version = "~> 2.0"
    }
  }

  backend "s3" {
    key = "rustyip/terraform.tfstate"

    # Cloudflare R2 (S3-compatible)
    # bucket, endpoint, access_key, secret_key are passed via -backend-config
    region                      = "auto"
    skip_credentials_validation = true
    skip_metadata_api_check     = true
    skip_region_validation      = true
    skip_requesting_account_id  = true
    skip_s3_checksum            = true
    use_path_style              = true
  }
}

provider "vultr" {
  api_key = var.vultr_api_key
}
