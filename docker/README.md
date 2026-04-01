# Local Messenger Relay — Docker Setup

## Prerequisites

- [Docker 24+](https://docs.docker.com/get-docker/)
- [Docker Compose v2](https://docs.docker.com/compose/install/) (`docker compose` — note: no hyphen)

---

## Quick start (< 5 minutes)

**Step 1 — Configure environment**

```sh
cp docker/.env.example docker/.env
```

Open `docker/.env` and set at minimum:

```
SERVER_NAME=relay.example.com          # public hostname or IP clients will connect to
INVITE_SECRET=a-long-random-secret     # used to sign invite tokens — keep this private
```

**Step 2 — Build and start**

```sh
cd docker
docker compose up -d --build
```

The relay starts listening on UDP port 7443 (or `RELAY_PORT` if overridden).  
On the very first boot the entrypoint generates a self-signed TLS certificate and stores it in the `relay-data` volume.

**Step 3 — Create the first invite link**

```sh
docker compose exec relay localmessenger_server create-invite \
    --label "welcome" \
    --max-uses 10 \
    --expires-in 72h
```

The command prints a `localmessenger://join/…` URL.

**Step 4 — Share the invite link**

Send the printed `localmessenger://join/…` URL to the people who should join your relay.  
They paste it into the **Onboarding → Join relay** field in the Local Messenger desktop client.

---

## Data persistence

All persistent state (SQLite database, TLS certificate, and private key) lives inside the named Docker volume **`relay-data`**, mounted at `/data` inside the container.

| Path inside container | Contents |
|-----------------------|----------|
| `/data/relay.db` | SQLite message / device database |
| `/data/relay-cert.der` | Self-signed TLS certificate (DER) |
| `/data/relay-key.der` | Corresponding private key (DER) |

**Backing up the volume:**

```sh
docker run --rm \
    -v localmessenger_relay-data:/data:ro \
    -v "$(pwd)":/backup \
    debian:bookworm-slim \
    tar czf /backup/relay-data-backup.tar.gz -C /data .
```

**Restoring from backup:**

```sh
docker run --rm \
    -v localmessenger_relay-data:/data \
    -v "$(pwd)":/backup \
    debian:bookworm-slim \
    tar xzf /backup/relay-data-backup.tar.gz -C /data
```

---

## Creating invite links

```sh
docker compose exec relay localmessenger_server create-invite \
    --label      "team-alpha" \   # human-readable label shown in the client
    --max-uses   5             \   # how many devices may use this link
    --expires-in 24h               # duration: 30m, 6h, 72h, etc.
```

Omit `--max-uses` for an unlimited link.  
Omit `--expires-in` for a link that never expires.

---

## Registering devices manually

If you prefer to register a device without an invite link (e.g. for automated provisioning):

```sh
docker compose exec relay localmessenger_server register-device \
    --member-name  "Alice"            \
    --device-name  "Alice's laptop"   \
    --device-id    "<hex device id>"
```

The command prints the resulting member / device record that was stored in the database.

---

## Upgrading

1. Pull the latest source (or bump the image tag in `Dockerfile`).
2. Rebuild and restart — existing data in the volume is preserved:

```sh
cd docker
docker compose up -d --build
```

Docker Compose will rebuild the image, stop the old container, and start a new one against the same `relay-data` volume.

---

## Environment variable reference

| Variable | Default | Description |
|---|---|---|
| `SERVER_NAME` | `relay.local` | Public hostname / IP used as TLS SNI and in invite URLs |
| `INVITE_SECRET` | *(required)* | Secret for signing invite tokens — must be set |
| `RELAY_PORT` | `7443` | Host UDP port forwarded to the container |
| `BIND_ADDR` | `0.0.0.0:7443` | Address the server binds inside the container |
| `DB_PATH` | `/data/relay.db` | Path to the SQLite database file |
| `CERT_PATH` | `/data/relay-cert.der` | Path to the TLS certificate (DER) |
| `KEY_PATH` | `/data/relay-key.der` | Path to the TLS private key (DER) |
| `RATE_WINDOW_SECONDS` | `60` | Rolling window for rate-limit counters |
| `PEER_FRAME_LIMIT` | `120` | Max frames per peer per rate window |
| `BLOB_REQUEST_LIMIT` | `32` | Max blob requests per peer per rate window |
| `BLOB_CHUNK_BYTE_LIMIT` | `20971520` | Max blob bytes per peer per rate window (20 MiB) |
| `HEALTH_CHECK_LIMIT` | `12` | Max health-check requests per peer per rate window |

See `docker/.env.example` for a ready-to-copy template.