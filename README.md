# Local Messenger

Secure local-first messenger for a small trusted group.

> [!WARNING]
> **This software has not undergone a formal security audit.**
> It is intended for small trusted groups who know each other in person.
> Do not use for high-risk communications until an independent audit is complete.
> See [docs/threat-model.md](docs/threat-model.md) for the full threat model and known limitations.

## Installation

Pre-built installers are published with every tagged release on GitHub:  
**<https://github.com/Rimus0cod/Local_sms/releases/latest>**

> **v1.0.0** — First public release. Includes group chats with sender-key fan-out, encrypted media transfer, Docker relay, and durable message queues.

### Windows

1. Download `LocalMessenger_x.x.x_x64-setup.exe` (NSIS installer) **or** `LocalMessenger_x.x.x_x64_en-US.msi` (MSI package).
2. Run the installer and follow the on-screen prompts.
3. Launch **Local Messenger** from the Start Menu or Desktop shortcut.

### macOS

1. Download `LocalMessenger_x.x.x_x64.dmg` (or `_aarch64.dmg` for Apple Silicon).
2. Open the `.dmg` file and drag **Local Messenger** into your **Applications** folder.
3. On first launch macOS may show a Gatekeeper warning — open **System Settings → Privacy & Security** and click **Open Anyway**.

### Linux

**AppImage** (works on most distributions, no installation required):

```bash
chmod +x LocalMessenger_x.x.x_amd64.AppImage
./LocalMessenger_x.x.x_amd64.AppImage
```

**Debian / Ubuntu package:**

```bash
sudo dpkg -i localmessenger_x.x.x_amd64.deb
```

---

## Quick Start

1. **Launch the app** — the main window opens with an empty contact list.
2. **First launch** — the app automatically generates your device identity (Ed25519 key pair). No account or server registration is required.
3. **Invite someone:**
   - Go to **Settings → Create Invite Link**.
   - Copy the generated link and send it to the other person via any channel (email, Signal, etc.).
4. **Join with an invite link (recipient side):**
   - Open the app, choose **File → Join with Invite Link**, and paste the link you received.
   - The app will complete the cryptographic handshake and add the contact automatically.
5. **Verify your contact** — to confirm you are not talking to an impersonator, compare the **Safety Number** or scan the **QR code** under **Settings → Verification**.  Both sides must see identical values.
6. **Start chatting!** — select the contact from the list and send your first message.

---

## Self-hosted Relay (optional)

Without a relay the app communicates **peer-to-peer over LAN only** (same Wi-Fi or local network segment).  If you and your contacts are on different networks and need to reach each other over the internet you can run the bundled QUIC relay server.

See **[docs/server-relay.md](docs/server-relay.md)** for full setup instructions, environment variables, and example systemd / Docker configurations.

---

## Build from Source

### Prerequisites

| Tool | Minimum version |
|------|----------------|
| [Rust](https://rustup.rs/) | 1.77 |
| [Node.js](https://nodejs.org/) | 20 |
| Cargo `tauri-cli` | bundled via npm scripts |

**Linux** — install system libraries first:

```bash
sudo apt-get update
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

**macOS** — Xcode Command Line Tools are sufficient (`xcode-select --install`).

**Windows** — install the [Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) and [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/).

### Production build

```bash
git clone https://github.com/Rimus0cod/Local_sms
cd Local_sms
npm install --prefix apps/tauri-client
cargo tauri build --manifest-path apps/tauri-client/src-tauri/Cargo.toml
```

Finished installers are placed in `apps/tauri-client/src-tauri/target/release/bundle/`.

### Development mode

```bash
npm run tauri:dev --prefix apps/tauri-client
```

This starts the Vite dev server and the Tauri shell with hot-reload for the frontend.

---

## Monorepo layout

```text
.
├── apps/
│   └── tauri-client/
├── crates/
│   ├── core/
│   ├── crypto/
│   ├── discovery/
│   ├── messaging/
│   ├── storage/
│   └── transport/
└── docs/
```

## Current status

- `STEP 1 — Crypto Layer` in `crates/crypto`
- `STEP 2 — Device Model` in `crates/core`
- `STEP 3 — Discovery Layer` in `crates/discovery`
- `STEP 4 — Transport Layer` in `crates/transport`
- `STEP 5 — Secure Session Integration` in `crates/messaging`
- `STEP 6 — Storage Layer` in `crates/storage`
- `STEP 7 — Messaging Engine` in `crates/messaging`
- `STEP 8 — Group Messaging` in `crates/messaging`
- `STEP 9 — Tauri Client` in `apps/tauri-client`
- `STEP 10 — Security Hardening` across `crates/crypto`, `crates/messaging`, and `docs/`
- `SPRINT 1 — QUIC Relay Server with LAN Fallback` across `apps/localmessenger_server`, `crates/server_protocol`, `crates/messaging`, and `apps/tauri-client`
- `SPRINT 2 — Offline Queue + Signed Invites + Client Onboarding` across `apps/localmessenger_server`, `crates/server_protocol`, and `apps/tauri-client`
- `SPRINT 3 — Media Files` across `apps/localmessenger_server`, `crates/server_protocol`, and `apps/tauri-client`
- `SPRINT 4 — Statuses + Notifications + Voice` in `apps/tauri-client`
- `SPRINT 5 — UI Polish + Hardening` across `apps/localmessenger_server` and `apps/tauri-client`

## Detailed crate and app status

- `crates/crypto` contains the security foundation for device-to-device encrypted sessions.
- `crates/core` contains member/device domain models, multi-device support, safety numbers, and QR-based verification payloads.
- `crates/discovery` contains LAN peer advertisements, TXT record encoding/decoding, peer registry maintenance, and an mDNS announce/browse runtime.
- `crates/transport` contains QUIC endpoints, self-signed transport identities, certificate pinning, framed uni-stream messaging, and reconnect policy support.
- `crates/messaging` contains the secure-session handshake, a reliable pairwise messaging engine, and a group sender-key layer with epoch rotation.
- `crates/storage` contains encrypted-at-rest SQLite persistence for devices, peers, message blobs, and local key material.
- `apps/tauri-client` contains a Tauri 2 + React + Zustand desktop client with a live Rust backend for direct secure chats, peer surfaces, and device verification.
- `apps/tauri-client` now also exposes delivery glyphs, online indicators, tray notifications, and voice-note recording/playback on top of the existing media flow.
- `apps/tauri-client` now also exposes reply/forward/reaction actions, PDF previews, and updater status surfaced from the desktop backend.
- `apps/localmessenger_server` contains a QUIC relay server with SQLite registry, challenge-response device auth, and manual registration CLI commands.
- `apps/localmessenger_server` now also enforces per-device in-memory rate limiting for relay and blob operations.
- `crates/server_protocol` contains the shared relay auth and forwarding wire types.

## Crypto layer scope

- X25519 identity keys for key agreement
- Ed25519 signed prekey authentication
- one-time prekeys
- X3DH-style session bootstrap
- Double Ratchet message encryption/decryption
- AES-256-GCM message protection
- HKDF-SHA256 derivation

## Device model scope

- member profiles with multiple devices
- device ownership validation
- pending/verified trust states
- safety number generation and verification
- QR verification payload encode/decode and matching

## Discovery layer scope

- mDNS presence advertisement with `libmdns`
- LAN peer browse loop with `mdns`
- TXT record codec for peer metadata
- peer registry with stale-peer expiration
- broadcasted peer add/update/expire events

## Transport layer scope

- QUIC transport with `quinn`
- self-signed peer transport certificates with explicit certificate pinning
- endpoint bind/listen and incoming connection accept
- outgoing connect with retry/backoff policy
- framed uni-stream transport messages over QUIC

## Secure session scope

- handshake exchange over QUIC `Handshake` frames
- binding responder device identity to its prekey bundle and pinned transport certificate
- X3DH bootstrap integrated with transport session setup
- Double Ratchet encrypted payload send/receive over `TransportConnection`
- deterministic session transcript binding used as associated data for encrypted messages

## Storage layer scope

- SQLite persistence via `sqlx`
- AES-256-GCM encryption for all stored record blobs
- encrypted local storage for device snapshots, peer snapshots, and message records
- encrypted persistence for local identity and prekey material
- deterministic hashed lookup keys so plaintext identifiers are not used as row keys

## Messaging engine scope

- encrypted user-message and ACK envelopes over `SecureSession`
- per-session delivery ordering with out-of-order buffering
- duplicate-safe retry for unacknowledged messages
- pending outbound queue and ACK-driven completion
- message id validation and protocol versioning

## Group messaging scope

- Signal-style sender-key foundation with a per-sender chain key and signing key
- sender-key distribution payloads ready to be carried over pairwise secure sessions
- encrypted group-message envelopes with per-sender signatures
- out-of-order group-message decryption support for the same sender chain
- epoch rotation helpers for member addition and member removal

## Tauri client scope

- Tauri 2 desktop shell with a Rust command layer
- React + Zustand frontend with chat list, chat window, LAN peer panel, and verification workspace
- direct desktop chat sends routed through live QUIC + secure-session + messaging-engine runtime sessions
- browser fallback backend so the UI can be developed without the desktop shell
- light and dark themes plus English and Russian interface copy
- verification actions wired to the real `crates/core` safety-number and QR validation logic
- group desktop surface intentionally remains staged until sender-key fan-out is wired end-to-end

## Security hardening scope

- explicit sender-key replacement rejection inside the same group epoch
- duplicate/replayed group-message rejection plus local duplicate group message-id guard
- bounded replay metadata retention for pairwise messaging engine state
- forward-secrecy state snapshots for Double Ratchet and secure-session validation tests
- rotation reasons for manual rekey and device-compromise driven epoch changes

## Relay server scope

- QUIC relay server with manual device registration
- Ed25519 challenge-response device auth
- opaque encrypted peer-frame forwarding for online recipients
- persistent store-and-forward queue for offline recipients
- HMAC-SHA256 signed invite links with max-use and expiry tracking
- desktop relay config with direct-LAN fallback
- device-registration bundle export and invite onboarding flow in the desktop client
- chunked encrypted blob upload/download for media up to 5 MB
- direct QUIC handoff for larger files
- desktop photo preview rendering for media messages
- PDF preview rendering for document messages
- Telegram-style reply/forward/reaction controls in the desktop client
- release-time Tauri updater artifact generation

## Important note

This uses audited primitive libraries, and the current stack now includes reliable pairwise delivery plus a group sender-key foundation on top of encrypted sessions. A secure product still requires durable pending-queue persistence across restarts, authenticated sender-key distribution over real fan-out flows, attachment storage, and end-to-end threat validation before real deployment.

Sprint 3 adds encrypted relay blob storage for small media plus direct QUIC transfer for larger files. The desktop client now renders photo previews, but real cross-device relay-backed media exchange is still only fully exercised through the backend flow rather than a full remote peer directory.

## Useful commands

```bash
cargo test -p localmessenger_crypto
cargo test
npm install --prefix apps/tauri-client
npm run build --prefix apps/tauri-client
```

Relay setup details and environment examples live in [docs/server-relay.md](docs/server-relay.md).