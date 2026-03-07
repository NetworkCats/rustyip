# Installation

### Required GitHub Secrets

Go to **Settings > Secrets and variables > Actions** in your GitHub repository and add the following secrets.

If you are using environments, add them under the `production` environment (**Settings > Environments > production**).

#### Terraform State (S3-compatible backend)

| Secret | Description |
|---|---|
| `TF_STATE_ACCESS_KEY` | Access key for the S3-compatible state backend (e.g. Cloudflare R2) |
| `TF_STATE_SECRET_KEY` | Secret key for the S3-compatible state backend |
| `TF_STATE_BUCKET` | Bucket name where Terraform state is stored |
| `TF_STATE_ENDPOINT` | S3-compatible endpoint URL (e.g. `https://<account_id>.r2.cloudflarestorage.com`) |

#### Infrastructure

| Secret | Description |
|---|---|
| `VULTR_API_KEY` | Vultr API key for provisioning VPS and firewall resources |
| `SSH_PUBLIC_KEY` | Public key deployed to the VPS for SSH access |
| `SSH_PRIVATE_KEY` | Corresponding private key used by the pipeline to connect via SSH |

#### Application

| Secret | Description |
|---|---|
| `SITE_DOMAIN` | Production domain name (e.g. `ip.nc.gy`) |
| `DB_UPDATE_URL` | URL to download the MMDB database from |

#### TLS

| Secret | Description |
|---|---|
| `ORIGIN_CERT` | Cloudflare Origin certificate (PEM) for TLS termination at HAProxy |
| `ORIGIN_KEY` | Corresponding private key (PEM) for the Origin certificate |

#### Docker Hub

| Secret | Description |
|---|---|
| `DOCKERHUB_USERNAME` | Docker Hub username used for image push and pull |
| `DOCKERHUB_TOKEN` | Docker Hub access token for authentication |

### Deploying

```bash
git tag v1.0.0
git push origin v1.0.0
```

This triggers the full pipeline. Monitor progress under the **Actions** tab in the repository.

### Infrastructure Details

- **VPS**: Vultr High Frequency (`vhf-2c-2gb`), Debian 13, region `dfw` (Dallas)
- **Firewall**: SSH open, HTTPS restricted to Cloudflare IP ranges
- **Deployment**: Blue-green with HAProxy traffic switching
- **First deploy**: Automatically applies base OS hardening (UFW, fail2ban, sysctl tuning) and installs Docker
- **Subsequent deploys**: Only the application container is updated via blue-green swap
