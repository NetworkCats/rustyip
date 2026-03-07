# Installation

## Prerequisites

- [Docker](https://docs.docker.com/get-docker/) and [Docker Compose](https://docs.docker.com/compose/install/)
- A [Cloudflare](https://www.cloudflare.com/) account with your domain proxied through it
- A Cloudflare Origin certificate for TLS between Cloudflare and your server

For building from source without Docker, you also need [Rust](https://rustup.rs/) 1.93.1+.

This project is designed to run behind Cloudflare. The application relies on the `CF-Connecting-IP` header to identify clients, and the included HAProxy configuration assumes Cloudflare as the TLS edge.

## Quick Start with Docker Compose

This is the recommended way to deploy RustyIP. It runs the application behind HAProxy with TLS termination and rate limiting.

### 1. Configure environment

```bash
cp .env.example .env
```

Edit `.env` and set at minimum:

```
SITE_DOMAIN=your-domain.example.com
```

### 2. Provide TLS certificates

Generate an Origin certificate in the Cloudflare dashboard under **SSL/TLS > Origin Server**. Place the combined certificate and key in PEM format at `haproxy/certs/origin.pem`:

```bash
mkdir -p haproxy/certs
cat origin-cert.pem origin-key.pem > haproxy/certs/origin.pem
```

The file must contain the certificate followed by the private key.

### 3. Start the stack

A prebuilt image is available on Docker Hub at [`networkcat/rustyip`](https://hub.docker.com/r/networkcat/rustyip). Update the `app` service in `docker-compose.yml` to use it:

```yaml
app:
  image: networkcat/rustyip
```

Then start the stack:

```bash
docker compose up -d
```

If you prefer to build from source instead, keep the default `build: .` and run:

```bash
docker compose up -d --build
```

The service will be available on port 443 (HTTPS). The MMDB database is downloaded automatically on first start and updated periodically (every 24 hours by default).

### 4. Configure Cloudflare

In the Cloudflare dashboard for your domain:

1. Add a DNS A record pointing to your server IP with the proxy enabled (orange cloud).
2. Set **SSL/TLS** mode to **Full (strict)**.

## Local Development

```bash
cp .env.example .env
mkdir -p data
curl -fsSL -o data/Merged-IP.mmdb \
  "https://github.com/NetworkCats/Merged-IP-Data/releases/latest/download/Merged-IP.mmdb"
cargo run
```

The server starts at `http://localhost:3000` by default. Set `DEV_MODE=true` in `.env` to use a fallback IP when the `CF-Connecting-IP` header is absent.

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `LISTEN_ADDR` | `0.0.0.0:3000` | Address and port the server listens on |
| `DB_PATH` | `data/Merged-IP.mmdb` | Path to the MMDB database file |
| `DB_UPDATE_URL` | *(GitHub release URL)* | URL to download the MMDB database from |
| `DB_UPDATE_INTERVAL_HOURS` | `24` | How often to check for database updates |
| `SITE_DOMAIN` | `localhost` | Domain name used for display and metadata |
| `DEV_MODE` | `false` | Uses `1.1.1.1` as fallback when `CF-Connecting-IP` is absent |
| `CERT_PATH` | `./haproxy/certs` | Path to the TLS certificate directory for HAProxy |

## Automated CI/CD Deployment

For production deployment with Terraform and Ansible via GitHub Actions, see [deploy/INSTALL.md](deploy/INSTALL.md).
