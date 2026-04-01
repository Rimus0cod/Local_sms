# Changelog

All notable changes to this project will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

- Anti-entropy sync between multiple devices owned by the same member
- Persistent pairwise replay-protection window across app restarts
- End-to-end sender-key fan-out over real network sessions (currently demo-loopback)
- Encrypted mDNS discovery beacons (currently plaintext TXT records)
- Encrypted export of local message history

---

## [1.0.0] - 2024-04-01

### Added

#### Core Cryptography (`crates/crypto`)

- Ed25519 identity keypair generation (`IdentityKeyPair::generate`) using
  `ed25519-dalek`; signing and public-key export with `sign_message` /
  `signing_public`.
- X25519 key agreement and X3DH-style session bootstrap (`x3dh_initiate` /
  `x3dh_respond`) for pairwise session establishment.
- Double Ratchet implementation (`DoubleRatchet`) with AES-256-GCM message
  encryption and decryption, out-of-order message-key storage
  (`skipped_message_keys`, bounded by `MAX_SKIP = 64`), and per-message
  random nonce generation via `OsRng`.
- HKDF-SHA256 key derivation helpers (`root_kdf`, `chain_kdf`) with
  domain-separated constants (`localmessenger/` prefix family).
- One-time prekey bundle support (`prekey.rs`) for X3DH.
- `RatchetStateSnapshot` for forward-secrecy validation in unit tests.
- `CryptoError::ReplayOrDuplicateMessage` returned by `DoubleRatchet::decrypt`
  when a message number has already been consumed.

#### Device and Member Model (`crates/core`)

- `MemberId` and `DeviceId` validated identifier types.
- `Device` struct representing a single device with ownership, trust state
  (`Pending` / `Verified`), and prekey bundle.
- Multi-device member profiles with per-device trust tracking.
- Safety-number generation: HKDF-SHA256 over both devices' Ed25519 public
  keys, formatted as groups of digits for out-of-band comparison.
- QR verification payload encode/decode and matching logic.

#### Discovery Layer (`crates/discovery`)

- mDNS presence advertisement using `libmdns`; service name
  `_rimus-chat._udp.local`.
- LAN peer browse loop using the `mdns` crate.
- TXT record codec for encoding and decoding peer metadata (device ID,
  display name, transport endpoint, capabilities).
- Peer registry with stale-peer expiry and add/update/expire event emission.

#### Transport Layer (`crates/transport`)

- QUIC transport endpoints (bind/listen/connect) using `quinn`.
- Self-signed per-device transport certificates with explicit certificate
  pinning on outgoing connections.
- Framed uni-stream messaging over QUIC with versioned frame types.
- Outgoing connection retry with configurable backoff policy.

#### Secure Session and Messaging Engine (`crates/messaging`)

- `SecureSession` constructed from `HandshakeResult`; wraps Double Ratchet
  over a live QUIC `TransportConnection`.
- X3DH + Double Ratchet handshake exchange (`src/handshake.rs`): initiator
  sends `SecureSessionRequest`; responder verifies device identity, transport-
  certificate fingerprint, and X3DH material before returning
  `SecureSessionResponse`.
- `MessagingEngine` for reliable pairwise delivery: delivery-order sequencing,
  out-of-order arrival buffering (`buffered_incoming: BTreeMap`), duplicate-
  safe retry for unacknowledged outbound messages, and encrypted ACK envelopes.
- `OutgoingMessage` and `DeliveredMessage` typed message structs with
  `message_id`, `conversation_id`, `delivery_order`, `sent_at_unix_ms`,
  `kind`, and `body` fields.
- `PendingQueueSnapshot` for exporting and restoring in-flight outbound
  message state across process restarts (`export_pending_queue` /
  `restore_pending_queue`).
- `ReceiveOutcome` combining delivered messages and acknowledged message IDs
  in a single return value from `MessagingEngine::receive_next`.
- Signal-style group sender-key layer (`src/group.rs`):
  - `GroupSenderKeyDistribution` containing `group_id`, `epoch`,
    `sender_member_id`, `sender_device_id`, `distribution_id`,
    `chain_key_seed`, and `signing_public_key`.
  - `GroupEncryptedMessage` with AES-256-GCM ciphertext and Ed25519 signature
    over `(group_id, epoch, distribution_id, message_id, message_number,
    ciphertext)`.
  - `GroupSession` managing both the local sender chain and all remote sender
    chains for the current epoch.
  - `LocalSenderKeyState` with per-sender chain key ratchet and Ed25519
    signing key; `RemoteSenderKeyState` with skipped-message-key map for
    out-of-order decryption (bounded by `MAX_GROUP_SKIP`).
  - `GroupMembership` with duplicate-device guard and 8-member hard cap
    (`MAX_GROUP_PARTICIPANTS`).
  - Epoch 0 bootstrapping: `GroupSession::create` generates the first local
    sender key and membership snapshot.
  - Epoch rotation helpers: `rotate_for_member_addition`,
    `rotate_for_member_removal`, `rotate_for_device_compromise`, and
    `rotate_for_manual_rekey`; each returns a `GroupEpochRotation` with
    `previous_epoch`, `next_epoch`, `reason`, updated `membership`, and new
    `local_sender_key_distribution`.
  - `GroupRotationReason` enum: `MemberAdded`, `MemberRemoved`,
    `DeviceCompromised`, `ManualForwardSecrecyRefresh`.
  - `GroupDecryptedMessage` struct with full sender and epoch provenance.

#### Encrypted Storage Layer (`crates/storage`)

- `SqliteStorage` backed by `sqlx` with WAL journal mode and `Full`
  synchronous writes for durability.
- `AtRestCipher` (`src/cipher.rs`) providing AES-256-GCM encryption for every
  stored record; fresh `OsRng`-generated 12-byte nonce per record;
  namespace-scoped associated data
  (`localmessenger/storage/aad/v1 || namespace || lookup_key`).
- `StorageKey` (256-bit) type implementing `Zeroize`; never stored in plaintext
  alongside the database.
- Hashed (opaque) row lookup keys: `opaque_lookup_key()` applies SHA-256 with
  domain prefix `localmessenger/storage/index/v1`; no plaintext identifier
  appears in any database index.
- Encrypted tables: `device_snapshots`, `local_device_secrets`,
  `peer_snapshots`, `message_log`, `pending_outbound_queue`.
- `pending_outbound_queue` table with `(peer_key, message_key, delivery_order,
  encrypted_blob)` schema; indexed by `(peer_key, delivery_order)` for
  ordered retrieval; API: `upsert_pending_outbound`, `pending_outbound_for_peer`,
  `remove_pending_outbound`, `clear_pending_outbound_for_peer`.
- `StoredPendingOutbound` model carrying `peer_device_id`, `message_id`,
  `delivery_order`, and serialized `OutgoingMessage`.
- `plaintext.zeroize()` called immediately after deserialization in
  `AtRestCipher::decrypt`.

#### Relay Server (`apps/localmessenger_server`)

- QUIC relay server (`serve` subcommand) with SQLite device registry.
- Ed25519 challenge-response device authentication (`src/auth.rs`,
  `Authenticator`): server issues a random 32-byte nonce; device signs
  `(member_id || device_id || nonce)` with its identity key; server verifies
  against the registered public key; nonce consumed on first use;
  configurable TTL.
- Opaque encrypted peer-frame forwarding (`src/relay.rs`): relay reads only
  the routing header and forwards ciphertext blobs without parsing content.
- Persistent store-and-forward offline queue: encrypted frames held for
  offline recipients and drained after successful authentication.
- HMAC-SHA256 signed invite links (`src/invite.rs`, `InviteService`):
  `create_invite` signs `InviteClaims` with the server's `invite_secret`;
  `join_with_invite` verifies signature, `status == "active"`, TTL
  (`expires_at_unix_ms`), and use count (`used_count < max_uses`) before
  registering the device.
- Invite link embeds server TLS certificate in DER form; desktop client pins
  this certificate, preventing silent server redirection.
- `create-invite` CLI subcommand: `--label`, `--server-addr`, `--server-name`,
  `--cert`, `--ttl-seconds`, `--max-uses`.
- `list-invites` CLI subcommand.
- `register-device` CLI subcommand for manual device enrollment without an
  invite link.
- `disable-device` CLI subcommand: marks device as `disabled = true` in
  registry; `Authenticator::verify_response` rejects disabled devices.
- `list-devices` CLI subcommand.
- `export_device_registration` Tauri command for generating a device-
  registration bundle for manual enrollment.
- `gen-cert` CLI subcommand for generating a self-signed TLS certificate pair
  (DER format).
- Chunked encrypted blob upload/download protocol for media up to 5 MB
  (`server_blob_store` route).
- Direct QUIC handoff for files over 5 MB (`p2p_quic_direct` route).
- In-memory per-device rate limiting (`src/rate_limit.rs`): configurable
  rolling-window counters for peer frames (`--peer-frame-limit`), blob
  requests (`--blob-request-limit`), blob bandwidth
  (`--blob-chunk-byte-limit`), and health checks (`--health-check-limit`).
- `crates/server_protocol`: shared wire types for relay auth
  (`AuthHello`, `AuthChallenge`, `AuthResponse`, `AuthOk`), invite
  (`InviteClaims`, `JoinWithInvite`, `JoinAccepted`), and device registration
  (`DeviceRegistrationBundle`).

#### Tauri Desktop Client (`apps/tauri-client`)

- Tauri 2 desktop shell with a React + Zustand frontend; light and dark themes;
  English and Russian interface copy (i18n).
- `ClientState` Rust struct with live QUIC + secure-session + messaging-engine
  runtime; `SharedClientState` (`Arc<Mutex<ClientState>>`) shared across
  Tauri commands.
- Tauri command layer (`src/commands.rs`): `get_snapshot`, `send_message`,
  `send_media`, `verify_device`, `start_chat_with_peer`,
  `export_device_registration`, `preview_invite`, `accept_invite`,
  `check_for_updates`, `toggle_reaction`, `forward_message`.
- `ClientSnapshot` serialized to the frontend on every state change:
  `transport_status`, `server_status`, `auth_status`, `active_route`,
  `notifications`, `local_profile`, `chats`, `peers`, `verification`,
  `onboarding`, `updater`.
- `ChatThreadView` with `id`, `title`, `summary`, `presence_label`,
  `presence_state`, `unread_count`, `security_label`, `kind`, `participants`,
  `messages`.
- `MessageView` with `id`, `author`, `body`, `timestamp_label`, `direction`,
  `delivery_state`, `forwarded_from`, `reply_preview`, `reactions`,
  `attachments`.
- `DeliveryStateView` enum: `Queued`, `Sent`, `Delivered`, `Seen`; seen-
  delivery icon styling in chat window.
- `MessageAttachmentView` with `id`, `file_name`, `mime_type`, `size_label`,
  `transfer_route`, `status_label`, `preview_data_url`, `blob_id`,
  `upload_progress`.
- Upload progress streamed to the frontend via Tauri events from
  `src/media.rs`.
- Photo preview rendering for image message attachments.
- PDF preview rendering for document attachments.
- Voice-note recording and playback integrated into the chat window.
- Inline auto-scroll to the latest message on new incoming or outgoing
  messages.
- Reply, forward, and reaction controls (Telegram-style) in the chat window.
- `VerificationWorkspaceView` and `VerificationDeviceView` with `safety_number`
  and `qr_payload_hex`; wired to real `crates/core` safety-number and QR
  validation.
- `PeerView` list in `ClientSnapshot::peers`; `refresh_peer_discovery`
  command refreshes mDNS results on demand.
- `OnboardingView` with 4-step wizard (`OnboardingPanel`): configure profile →
  choose relay or LAN-only → paste invite link → confirm connection.
- `InvitePreviewView` showing `invite_id`, `label`, `server_addr`,
  `server_name`, `expires_at_label`, `max_uses` before the user confirms
  joining.
- `UpdaterView` surfacing `current_version`, `channel`, `status_label`,
  `last_checked_label`, `can_auto_update`, `feed_url`.
- `NotificationCenterView` with `tray_label`, `unread_count`, `last_event`.
- System-tray icon with unread-message badge and notification pop-ups via
  `src/tray.rs`.
- `TransportStatusView` surfacing `discovery_mode`, `transport_mode`,
  `crypto_mode`, `storage_mode`, `server_status`, `auth_status`,
  `active_route` to the frontend.
- `GroupChatRuntime` for group message fan-out in the desktop client; group-
  specific send/receive test (`group_send_message_fan_out_delivers_and_
  receives_replies`).
- Browser fallback backend for UI development without the Tauri shell;
  explicitly not a production transport path.
- Tauri bundle generation configured to produce updater artefacts
  (`.nsis.zip`, `.msi.zip`, `.tar.gz`, `.app.tar.gz`) for release publishing.
- `LOCALMESSENGER_UPDATER_FEED` and `LOCALMESSENGER_UPDATER_CHANNEL`
  environment variables for updater configuration.

#### Docker and Infrastructure

- Multi-stage Docker image (`docker/Dockerfile`): Rust builder stage producing
  a minimal Debian-slim runtime image for `localmessenger_server`.
- `docker/docker-compose.yml` with named `relay-data` volume for SQLite
  database, TLS certificate, and private key.
- `docker/entrypoint.sh`: auto-generates a self-signed certificate on first
  boot if none is present in the data volume.
- `docker/.env.example` environment template with documented variables:
  `SERVER_NAME`, `INVITE_SECRET`, `RELAY_PORT`, `BIND_ADDR`, `DB_PATH`,
  `CERT_PATH`, `KEY_PATH`, and all rate-limit parameters.
- `docker/README.md`: quick-start guide (under 5 minutes), data-persistence
  reference, backup and restore procedures, upgrade instructions, and
  environment-variable reference table.
- GitHub Actions CI/CD pipeline (`.github/workflows/`): matrix build across
  Windows, macOS (x86-64 + Apple Silicon), and Linux; runs `cargo test`
  and produces Tauri installer artefacts (`*.exe`, `*.msi`, `*.dmg`,
  `*.AppImage`, `*.deb`) on every tagged release.
- Monorepo `Cargo.toml` workspace with all crates and apps in a single
  lock file.

#### Documentation

- `README.md`: Installation section (Windows NSIS/MSI, macOS DMG, Linux
  AppImage/deb), Quick Start walkthrough, Self-hosted Relay section,
  Build from Source instructions, monorepo layout diagram, and per-crate
  status summary.
- `docs/protocol.md`: full protocol specification covering group creation,
  invite flow, LAN discovery, join handshake, message envelope format,
  pairwise delivery semantics, presence events, epoch rekey flow, local
  storage model, desktop client surface, and hardening rules.
- `docs/server-relay.md`: relay server CLI reference, all environment
  variables, desktop env configuration, and operational notes.
- `docs/threat-model.md`: comprehensive English threat model covering scope
  and assumptions, seven guaranteed security properties, six non-properties,
  ten threat analyses with summary table and detailed descriptions, full
  implementation-status mapping, known limitations, and security contact.
- `docs/user-guide.md`: end-user guide covering installation, first launch,
  direct chat setup, device verification (QR and safety number), group
  chats, relay server setup, file and media transfer, LAN-only mode, FAQ,
  and security best practices.
- `docs/architecture.md`: high-level architecture overview.
- `docs/roadmap.md`: development roadmap.

### Changed

- Group sender-key import made idempotent for identical distributions:
  `GroupSession::import_sender_key` accepts a re-delivery of the exact same
  distribution (same `distribution_id`) without error; conflicting
  distributions from the same sender in the same epoch are rejected as a hard
  protocol error rather than silently accepted.
- Relay transport now preferred over direct LAN in the default transport order
  (`server_relay,direct_lan`); can be overridden via
  `LOCALMESSENGER_TRANSPORT_ORDER`.
- Media upload previously blocked by a staged guard; guard removed so group
  media messages flow through the relay blob store end-to-end.
- `pending_outbound_queue` delivery order changed from `INTEGER PRIMARY KEY
  AUTOINCREMENT` to an explicit `delivery_order INTEGER NOT NULL` column
  indexed per peer, enabling correct multi-peer ordering.

### Security

- **AGPL-3.0 license** applied to all source files; copyleft ensures that
  modifications to the network-facing relay and client code must be shared.
- **Replay protection — pairwise layer:** `MessagingEngine` tracks a bounded
  window of seen `(message_id, delivery_order)` pairs
  (`incoming_order_index`, `incoming_message_orders`); any frame with a
  replayed `message_id` or a `delivery_order` below
  `next_expected_incoming_order` is rejected before decryption.
- **Replay protection — group layer:** `RemoteSenderKeyState::ensure_message_
  fresh` checks `message_id_index` and `message_number_index` independently;
  replayed group messages are rejected within the current epoch.
- **Same-epoch sender-key replacement is a hard protocol error:**
  `GroupSession::import_sender_key` returns an error when a conflicting
  distribution for the same sender device already exists in the current epoch;
  this closes a potential downgrade/substitution vector.
- **Durable pending queue survives restarts:** `PendingQueueSnapshot` is
  exported to `SqliteStorage::upsert_pending_outbound` (AES-256-GCM encrypted)
  so no in-flight message is silently dropped across a process restart.
- **AES-256-GCM blob encryption for media:** relay-stored attachment blobs are
  encrypted on the sender device before upload; the encryption key is
  distributed only inside the end-to-end encrypted message envelope; the relay
  never possesses a plaintext copy of any attachment.
- **HMAC-SHA256 signed invite links:** `InviteService` uses the server's
  `invite_secret` to sign `InviteClaims`; `verify_invite_link` checks the MAC
  plus status, expiry, and use-count before registering a device.
- **Ed25519 challenge-response relay auth:** single-use, TTL-bounded nonce;
  disabled devices are rejected before signature verification.
- **Forward-secrecy state snapshots:** `RatchetStateSnapshot` and
  `SecureSession` snapshot types used in unit tests to assert that ratchet
  state advances on every message and that old keys are not recoverable from
  current state.
- **Bounded replay metadata window:** `MAX_TRACKED_INCOMING_ORDERS` caps the
  in-memory replay set; `prune_incoming_history` prevents unbounded growth.
- **`zeroize` on sensitive buffers:** `AtRestCipher::encrypt` and `::decrypt`
  call `plaintext.zeroize()` after use; `StorageKey` implements `Zeroize`.
- **Opaque row lookup keys:** SHA-256 hashed identifiers with a domain-
  separation prefix prevent plaintext member/device/message IDs from appearing
  in the SQLite index, reducing information leakage if the database file is
  inspected without the storage key.
- **Per-device relay rate limiting:** in-memory per-session counters bound the
  relay's utility as a relay-amplification or DoS vector.
- **Browser fallback explicitly dev-only:** no production transport path
  exists through the browser backend; IPC restricted to registered Tauri
  commands.

### Infrastructure

- **GitHub Actions CI/CD:** matrix workflow builds and tests on
  `ubuntu-latest`, `macos-latest`, and `windows-latest`; Tauri bundles
  (`*.exe`, `*.msi`, `*.dmg`, `*.AppImage`, `*.deb`) published as release
  artefacts on every version tag.
- **Rust workspace:** unified `Cargo.toml` and `Cargo.lock` across all crates
  (`crypto`, `core`, `localmessenger_core`, `discovery`, `transport`,
  `messaging`, `storage`, `server_protocol`) and apps
  (`localmessenger_server`, `localmessenger_cli`, `tauri-client`).
- **Docker multi-stage build:** final image based on `debian:bookworm-slim`;
  binary copied from the Rust builder stage; image size minimised by excluding
  build toolchain from the runtime layer.
- **`docker compose up -d --build`** is the single command needed to start a
  fully operational relay with auto-generated TLS certificate.
- **SQLite WAL mode + `Full` synchronous writes** in `SqliteStorage` for
  durability; in-memory mode for unit tests.
- **Tauri updater artefacts:** `tauri.conf.json` configured to produce
  `.nsis.zip`, `.msi.zip`, `.tar.gz` (Linux), and `.app.tar.gz` (macOS)
  signature bundles; `LOCALMESSENGER_UPDATER_FEED` points the desktop client
  to the update manifest URL.

---

[Unreleased]: https://github.com/Rimus0cod/Local_sms/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/Rimus0cod/Local_sms/releases/tag/v1.0.0