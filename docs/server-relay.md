# Server Relay Setup

Sprint 5 extends the self-hosted QUIC relay server and desktop client with server-side rate limiting, Telegram-style chat actions, PDF previews, and release-time updater artifacts for the Tauri shell.

## Included

- QUIC relay server with SQLite registry
- Ed25519 challenge-response auth using existing device signing keys
- Immediate forwarding of opaque encrypted peer frames
- Persistent offline queue for encrypted peer payloads
- Signed invite links using HMAC-SHA256
- Join-with-invite onboarding path for the desktop client
- Chunked upload/download protocol for encrypted blobs up to 5 MB
- Server-side storage of encrypted media blobs in SQLite
- In-memory per-device rate limiting for peer relay, blob requests, blob bandwidth, and health checks
- Desktop media routing:
  - `server_blob_store` for encrypted files up to 5 MB when relay is authenticated
  - `p2p_quic_direct` for larger files over direct QUIC
- Photo preview rendering in the desktop chat UI
- PDF preview rendering in the desktop chat UI
- Reply, forward, and reaction actions in the desktop chat UI
- Tauri bundle generation configured to produce updater artifacts for release publishing
- Desktop snapshot fields for `serverStatus`, `authStatus`, and `activeRoute`
- Desktop command `export_device_registration(path)` for manual enrollment

## Server CLI

```bash
cargo run -p localmessenger_server -- serve \
  --bind 0.0.0.0:7443 \
  --server-name relay.local \
  --cert /absolute/path/relay-cert.der \
  --key /absolute/path/relay-key.der \
  --db /absolute/path/localmessenger-relay.db \
  --invite-secret changeme \
  --rate-window-seconds 60 \
  --peer-frame-limit 120 \
  --blob-request-limit 32 \
  --blob-chunk-byte-limit 20971520 \
  --health-check-limit 12
```

```bash
cargo run -p localmessenger_server -- create-invite \
  --db /absolute/path/localmessenger-relay.db \
  --invite-secret changeme \
  --label "Home relay" \
  --server-addr 203.0.113.10:7443 \
  --server-name relay.local \
  --cert /absolute/path/relay-cert.der \
  --ttl-seconds 86400 \
  --max-uses 4
```

```bash
cargo run -p localmessenger_server -- list-invites \
  --db /absolute/path/localmessenger-relay.db
```

```bash
cargo run -p localmessenger_server -- register-device \
  --db /absolute/path/localmessenger-relay.db \
  --bundle /absolute/path/device-registration.json
```

```bash
cargo run -p localmessenger_server -- disable-device \
  --db /absolute/path/localmessenger-relay.db \
  --device-id alice-phone
```

```bash
cargo run -p localmessenger_server -- list-devices \
  --db /absolute/path/localmessenger-relay.db
```

## Desktop Env

```bash
LOCALMESSENGER_SERVER_ADDR=203.0.113.10:7443
LOCALMESSENGER_SERVER_NAME=relay.local
LOCALMESSENGER_SERVER_CERT_DER=/absolute/path/relay-cert.der
LOCALMESSENGER_SERVER_DEVICE_ID=rimus-laptop
LOCALMESSENGER_TRANSPORT_ORDER=server_relay,direct_lan
LOCALMESSENGER_UPDATER_FEED=https://updates.example.com/localmessenger/latest.json
LOCALMESSENGER_UPDATER_CHANNEL=stable
```

Notes:

- `LOCALMESSENGER_SERVER_CERT_DER` must be a DER-encoded certificate file.
- If relay connect/auth fails, the desktop client remains usable and falls back to `direct_lan`.
- Sprint 2 persists undelivered encrypted payloads on the server and drains them after the recipient authenticates.
- Sprint 3 stores encrypted media blobs on the server only for files up to 5 MB; larger files are handed off to a direct QUIC lane instead of relay storage.
- Invite links embed server address, server name, and the trusted DER certificate so the desktop client can onboard without an out-of-band cert path.
- `LOCALMESSENGER_UPDATER_FEED` only drives desktop updater status in this workspace build. Runtime install/download still needs the packaged app to enable the Tauri updater plugin.
- The server rate limiter is currently in-memory and scoped per authenticated device session window.
