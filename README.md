# Local Messenger

Secure local-first messenger for a small trusted group.

This repository now follows the target monorepo layout and has working implementations for:

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

- `crates/crypto` contains the security foundation for device-to-device encrypted sessions.
- `crates/core` contains member/device domain models, multi-device support, safety numbers, and QR-based verification payloads.
- `crates/discovery` contains LAN peer advertisements, TXT record encoding/decoding, peer registry maintenance, and an mDNS announce/browse runtime.
- `crates/transport` contains QUIC endpoints, self-signed transport identities, certificate pinning, framed uni-stream messaging, and reconnect policy support.
- `crates/messaging` contains the secure-session handshake, a reliable pairwise messaging engine, and a group sender-key layer with epoch rotation.
- `crates/storage` contains encrypted-at-rest SQLite persistence for devices, peers, message blobs, and local key material.
- `apps/tauri-client` contains a Tauri 2 + React + Zustand desktop client with a live Rust backend for direct secure chats, peer surfaces, and device verification.

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

## Important note

This uses audited primitive libraries, and the current stack now includes reliable pairwise delivery plus a group sender-key foundation on top of encrypted sessions. A secure product still requires durable pending-queue persistence across restarts, authenticated sender-key distribution over real fan-out flows, attachment storage, and end-to-end threat validation before real deployment.

## Useful commands

```bash
cargo test -p localmessenger_crypto
cargo test
npm install --prefix apps/tauri-client
npm run build --prefix apps/tauri-client
```
