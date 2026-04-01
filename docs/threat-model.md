# Local Messenger — Threat Model

**Version:** 1.0.0
**Date:** 2024-04-01
**Status:** Living document — updated with each major release

---

## Table of Contents

1. [Scope and Assumptions](#1-scope-and-assumptions)
2. [Security Properties Guaranteed](#2-security-properties-guaranteed)
3. [Properties NOT Guaranteed](#3-properties-not-guaranteed)
4. [Threat-by-Threat Analysis](#4-threat-by-threat-analysis)
5. [Implementation Status](#5-implementation-status)
6. [Known Limitations and Future Work](#6-known-limitations-and-future-work)
7. [Security Contact](#7-security-contact)

---

## 1. Scope and Assumptions

### 1.1 What Is Being Protected

Local Messenger is a local-first, end-to-end encrypted messenger designed for a small closed group
of people who know and trust one another. The following assets are considered sensitive and are
protected by the cryptographic and operational controls described in this document.

| Asset | Description | Storage location |
|-------|-------------|-----------------|
| **Plaintext messages** | Human-readable body of every message, including text, system events, and control payloads | Never persisted or transmitted in plaintext; decrypted only in the trusted process memory of the intended recipient |
| **Attachments** | Binary blobs (images, audio, PDF, arbitrary files) attached to messages | AES-256-GCM encrypted before upload to relay; the encryption key is carried only inside the encrypted message envelope |
| **Group membership** | The list of device identifiers that belong to a group, the current epoch number, and per-member sender-key distributions | Managed in-memory; persisted only inside AES-256-GCM encrypted SQLite records |
| **Device keys** | Ed25519 identity signing keys, X25519 ratchet keys, one-time prekeys, and local storage keys | Persisted only in the encrypted `local_device_secrets` SQLite table; never transmitted in plaintext |
| **Local message history** | The entire conversation log on a device | Stored as AES-256-GCM encrypted blobs in SQLite; row lookup keys are SHA-256 hashes so no plaintext identifier appears as a database index value |

### 1.2 Trust Boundary

Local Messenger is explicitly designed for **small, closed groups of ≤ 8 members** who know each
other in real life or have exchanged cryptographic trust anchors (QR codes or safety numbers)
through an out-of-band channel they already trust — an in-person meeting, phone call, or video
call.

This design choice has concrete security implications:

- Group metadata (membership lists, device IDs) is shared only among current members and is not
  discoverable by outsiders.
- Each group member is trusted to encrypt honestly for their own sender chain. A member who
  deliberately corrupts their ciphertext can be detected (group messages carry Ed25519 per-sender
  signatures), but a member can always send misleading plaintext content — social trust is a
  prerequisite that the cryptographic layer cannot replace.
- The app does **not** provide anonymity among members. All members know each other's device
  identifiers and real-time presence state.

### 1.3 Attacker Models

The following attacker models are considered **in scope**:

#### 1.3.1 Passive Network Adversary

An attacker who can record all traffic on the LAN segment or the internet path, including QUIC
UDP datagrams, mDNS announcements, and relay-server connections. This adversary has read-only
access to the wire and cannot modify or inject packets.

**Threat level:** Medium. All message payloads are end-to-end encrypted. The adversary can observe
connection metadata — which IP addresses contact the relay, connection timing, and approximate
message sizes — but cannot read message content.

#### 1.3.2 Active MITM Before Device Verification

An attacker who can intercept and modify QUIC connections at the network level, substituting their
own public keys during the X3DH handshake. This attack window exists from the moment a device
first discovers a new peer until both users complete QR or safety-number verification.

**Threat level:** High if verification is skipped; fully mitigated once verification is completed.
The only persistent defence against this class of attack is out-of-band key comparison. Once both
sides mark the device as "Verified," any prior MITM substitution would produce a detectable
safety-number mismatch.

#### 1.3.3 Compromised Relay Server

An attacker with full access to the relay server's database, process memory, and network stack.
They can read all stored data, inject messages into the offline queue, and observe all connection
metadata.

**Threat level:** Medium for message content (mitigated by E2EE); high for metadata. The relay
stores only opaque encrypted frames and cannot decrypt message content. However, it can observe
device IDs, connection timestamps, payload sizes, and the full communication graph of relay-
connected devices.

#### 1.3.4 Compromised Device

An attacker who has physical or remote access to a device that is or was a group member. They can
read the device's SQLite database files, take memory dumps, and access any cached key material.

**Threat level:** High for messages stored on the compromised device. Forward secrecy from the
Double Ratchet limits exposure of past ratchet states, but all messages persisted in SQLite are
accessible if the storage encryption key is recovered.

#### 1.3.5 Malicious Group Member (Post-Exclusion)

A group member who was legitimately part of the group but has since been removed. They retain all
messages and sender-key material from the period when they were a member and may attempt to
decrypt messages sent after their removal.

**Threat level:** Low for post-removal messages when epoch rotation is correctly executed. The
protocol mandates a `rotate_for_member_removal` epoch transition on every membership removal, and
the new sender-key distributions are delivered only to the remaining participants.

#### 1.3.6 Malicious Invite Link

An attacker who intercepts, forges, or replays an invite link in order to join the relay server
under a false identity, or to trick a legitimate user into connecting to an adversary-controlled
server.

**Threat level:** Low for properly configured invites, mitigated by HMAC-SHA256 signing plus
expiry and use-count enforcement. Each invite link also embeds the server's TLS certificate in DER
form, making it hard to silently redirect a user to a different server.

### 1.4 Explicitly Out-of-Scope Attacker Models

The following attacker models are **out of scope** for this version of the threat model:

- **Supply chain / OS compromise.** An attacker who can modify the app binary, operating system,
  or system libraries is out of scope. If the OS is fully compromised, no application-layer
  security mechanism is sufficient. Mitigations require reproducible builds, binary signing, and
  OS-level attestation — all of which are future work.
- **Side-channel attacks.** Power analysis, timing side channels on AES operations, and similar
  hardware-level attacks are out of scope.
- **Coercion / rubber-hose cryptanalysis.** An attacker who compels a user to reveal their storage
  key by force or legal order is out of scope.

---

## 2. Security Properties Guaranteed

### 2.1 Confidentiality

All messages and media blobs are encrypted with **AES-256-GCM** before leaving the sending device.
Pairwise messages use message keys derived by the Double Ratchet, which is seeded by X3DH.
Group messages use per-sender chain keys derived by HKDF-SHA256, each encrypted with AES-256-GCM
under a fresh random nonce. Media blobs stored on the relay are encrypted with a random AES-256-GCM
key that is carried only inside the encrypted message envelope; the relay server stores only the
opaque ciphertext and cannot access the plaintext of any message or attachment.

Only devices that hold the corresponding session keys can decrypt messages addressed to them.

### 2.2 Authentication

Device identity is anchored to an **Ed25519** keypair generated locally at first launch and never
transmitted in plaintext. The relay server authenticates devices through an Ed25519 challenge-
response protocol: the server issues a random 32-byte nonce, the device signs
`(member_id || device_id || nonce)` with its identity key, and the server verifies the signature
against the registered public key. The challenge nonce is single-use and TTL-bounded.

Group messages carry an Ed25519 signature over a payload covering
`(group_id, epoch, distribution_id, message_id, message_number, ciphertext)`, allowing every
recipient to confirm the message originated from the claimed sender.

Session bootstrap uses an **X3DH**-style handshake: the initiator uses the responder's published
identity key, signed prekey, and one-time prekey to derive a shared root key that seeds the Double
Ratchet. This provides mutual authentication of the key exchange and deniability at the session
level.

### 2.3 Forward Secrecy

The **Double Ratchet** algorithm ensures that past message keys cannot be recovered from the
current session state. Each message advances the symmetric chain key, and each DH ratchet step
generates a fresh X25519 ephemeral key pair, discarding the previous DH secret. An attacker who
compromises the current ratchet state cannot recover keys for any message that was encrypted
before the most recent ratchet advance. Forward secrecy is validated by state-snapshot tests in
`crates/crypto/src/ratchet.rs` and `crates/messaging/src/session.rs`.

### 2.4 Break-in Recovery

Because the Double Ratchet generates a new DH key pair on every ratchet step triggered by an
incoming message, compromise of the current ratchet keys is self-healing: as soon as the
legitimate party sends a new message (triggering a ratchet advance with a fresh ephemeral key),
the attacker is excluded from all subsequent message keys. The window of exposure after a key
compromise is limited to messages encrypted within the same ratchet epoch — typically a single
exchange of messages.

### 2.5 Replay Protection

Replay protection is enforced independently at two protocol layers:

**Pairwise layer** (`crates/messaging/src/engine.rs`, `MessagingEngine`): each incoming message
envelope carries a `delivery_order` sequence number and a unique `message_id`. The engine tracks
a bounded window of seen `(message_id, delivery_order)` pairs up to `MAX_TRACKED_INCOMING_ORDERS`.
Any frame with a `delivery_order` below `next_expected_incoming_order`, or with a `message_id`
already in the `incoming_order_index` set, is rejected before any decryption is attempted. Replayed
ACK envelopes are detected by the same mechanism.

**Group layer** (`crates/messaging/src/group.rs`, `RemoteSenderKeyState`): each group message
carries a `message_id` and a `message_number`. The `ensure_message_fresh` method checks both the
`message_id_index` (exact-match deduplication set) and the `message_number_index` (order-based
tracking) to reject replayed or duplicated group messages within the current epoch. Attempting to
replace an existing sender-key distribution for the same device in the same epoch is treated as a
hard protocol error rather than a silent update.

The Double Ratchet itself provides a third layer: attempting to decrypt a ciphertext with an
already-consumed message key returns `CryptoError::ReplayOrDuplicateMessage`.

### 2.6 Post-Removal Secrecy

When a member is removed from a group, `GroupSession::rotate_for_member_removal` increments the
epoch, generates entirely new `LocalSenderKeyState` material (new `chain_key_seed`, `signing_key`,
and `distribution_id`) for the local device, and produces a `GroupEpochRotation` with
`reason = GroupRotationReason::MemberRemoved { device_id }`. The resulting
`GroupSenderKeyDistribution` is distributed only to devices that remain in the post-removal
membership set. The removed device retains its old sender chain and can still decrypt messages
from the previous epoch (as it was a legitimate member then), but cannot produce or decrypt any
message from the new epoch.

### 2.7 Integrity

Every pairwise message ciphertext includes an AES-GCM authentication tag covering both the
ciphertext and the namespace-scoped associated data derived from the message header and session
transcript (`localmessenger/double-ratchet/v1 || associated_data || serialized_header`). Any
modification to the ciphertext or header causes decryption to fail with an authentication error;
no corrupted plaintext is ever returned to the application layer.

Group messages independently carry an Ed25519 signature verifiable against the sender's registered
signing public key. Recipients call `verify_group_message_signature` before attempting to decrypt
the ciphertext. Tampering with any component of the signed payload is detected and results in a
verification error.

### 2.8 Durable Pending Queue

Outbound messages that have not yet been acknowledged by the peer are stored in an encrypted
`pending_outbound_queue` SQLite table (`crates/storage/src/store.rs`,
`SqliteStorage::upsert_pending_outbound`). The `PendingQueueSnapshot` structure in
`crates/messaging/src/engine.rs` enables `MessagingEngine::export_pending_queue` and
`restore_pending_queue` to persist and restore the engine's in-flight state across process
restarts. No acknowledged message is silently dropped if the app crashes or the peer is
temporarily unreachable.

---

## 3. Properties NOT Guaranteed

### 3.1 Traffic Analysis Resistance

Local Messenger makes no attempt to obscure the timing, frequency, or size of messages. An
adversary monitoring network traffic — even without decrypting any content — can observe when
devices communicate, how often, and approximate message volumes. Padding, cover traffic, and
timing obfuscation are not implemented.

### 3.2 Anonymity Among Members

Group members are fully aware of each other's device identifiers and presence state. The protocol
is designed for a closed group of mutually known participants; anonymity sets, onion routing, and
similar anonymity-preserving mechanisms are out of scope.

### 3.3 Metadata Hiding from the Relay

The relay server authenticates each device by ID and therefore knows which device IDs are
registered, which are currently connected, and which device is sending frames to which other
device. The relay cannot read the plaintext of any message, but over time it can build a complete
communication graph of relay-connected devices. Operational metadata (connection timestamps, payload
sizes, frame counts) is also visible to the relay operator.

### 3.4 OS-Level Protection

If the operating system or a process running with OS-level privileges is compromised, an attacker
can extract keys from memory, read unencrypted SQLite WAL pages before they are flushed, or inject
code into the app process. No application-layer mechanism can prevent this class of attack. Full-
disk encryption and OS-level hardening are strongly recommended as complementary controls.

### 3.5 Availability Guarantee

The relay server can selectively drop, delay, or reorder messages without the sender or recipient
being aware in real time. The pairwise delivery engine retries unacknowledged messages indefinitely
(bounded by its retry budget and the durable SQLite queue), but a fully adversarial relay can deny
delivery indefinitely. There is no cryptographic proof of delivery and no third-party audit log to
detect selective message suppression.

### 3.6 Formal Verification and Security Audit

The cryptographic protocol and its implementation have not been formally verified or reviewed by an
independent security auditor. The primitive libraries used (`aes-gcm`, `ed25519-dalek`,
`x25519-dalek`, `hkdf`) are well-established audited implementations, but the protocol
composition — in particular the combination of X3DH, Double Ratchet, and the group sender-key
layer — has only been reviewed internally. A formal audit is the highest-priority security
improvement before any production deployment at scale.

---

## 4. Threat-by-Threat Analysis

### 4.1 Summary Table

| # | Threat | Risk | Attack Vector | Primary Mitigation | Residual Risk |
|---|--------|------|---------------|--------------------|---------------|
| 1 | MITM during device onboarding | **High** (before verification) / None (after) | Network-level key substitution during X3DH | QR code / Safety Number out-of-band verification | None if verification is completed correctly |
| 2 | LAN eavesdropping | **Low** | Passive packet capture on shared Wi-Fi | AES-256-GCM E2EE; all payloads opaque to observer | Traffic metadata (timing, size, peers) remains visible |
| 3 | Relay server compromise | **Medium** | Full server access (DB, memory, network) | Relay stores and forwards only opaque ciphertext | Communication graph and connection metadata exposed |
| 4 | Stale epoch after member removal | **Low** | Removed member retains old sender-key state | Mandatory epoch rotation on every member removal | Pre-removal messages remain readable by removed member |
| 5 | Device theft / physical access to SQLite | **High** | Read raw database files from stolen device | AES-256-GCM encrypted-at-rest storage; device revocation CLI | Accessible if storage key is also recovered |
| 6 | Forged or replayed invite link | **Low** | Intercept and replay invite URL | HMAC-SHA256 signed invite; expiry and use-count enforcement | Weak invite secrets reduce this guarantee |
| 7 | Message replay / duplicate injection | **Low** | Re-inject previously captured frames | Pairwise + group replay-rejection layers; Double Ratchet duplicate guard | Pre-restart replay window not yet persisted to disk |
| 8 | Key material exposure after app crash | **Low** | Memory forensics on core dump or swap | `zeroize` on sensitive buffers; encrypted SQLite; durable queue | OS-level memory forensics out of scope |
| 9 | Relay blocking delivery (availability attack) | **Medium** | Relay drops or delays frames for specific device | Retry queue; direct-LAN fallback; durable pending store | No cryptographic delivery proof; relay-only paths vulnerable |
| 10 | Browser fallback dev path used in production | **Low** | Developer serves static `dist/` without Tauri shell | Fallback is dev-only; production delivered only as Tauri bundle | Misconfigured non-production deployment risk |

---

### 4.2 Detailed Threat Descriptions

#### Threat 1 — MITM During Device Onboarding

**Attack description.**
During the initial X3DH handshake, an attacker positioned on the local network — or controlling
the relay — could substitute their own X25519 keys for either party's published keys. The attacker
then establishes two independent encrypted sessions (one with each legitimate device) and silently
relays all messages, reading and optionally modifying them in transit. Neither device detects the
substitution from protocol headers alone, because the ciphertext still decrypts successfully
against the MITM-established session keys.

**Why this matters.**
This is the most dangerous attack in the threat model because it can be mounted silently,
persists indefinitely, and defeats all other cryptographic guarantees. Every subsequent message
in the compromised session is readable by the attacker until the MITM is detected and the session
is torn down.

**Mitigation.**
The application surface exposes two out-of-band verification mechanisms, both implemented via the
real `crates/core` validation logic wired into the Tauri command layer
(`apps/tauri-client/src-tauri/src/state.rs`, `ClientState::verify_device`):

1. **QR Code** — One device renders its Ed25519 signing public key and device ID as a hex-encoded
   QR payload. The other device scans or pastes this payload. The verification logic compares the
   received public key against the key used to authenticate the current session. If an attacker
   substituted their own key, the QR scan produces a mismatch.
2. **Safety Number** — Both devices independently compute a short numeric fingerprint by applying
   HKDF-SHA256 to the concatenated Ed25519 public keys of both participants. If both users read
   the same six-digit groups aloud and they match exactly, no key substitution has occurred.

Once both parties complete verification, the trust state is recorded in the Tauri client and the
UI displays a "Verified" badge and security label. Subsequent session establishments are compared
against the verified key, and any change triggers a new verification requirement.

**Residual risk.**
None if verification is completed correctly and in a trusted channel. If verification is skipped,
performed carelessly (e.g., safety numbers compared by text message that the attacker could also
intercept), or if the QR is photographed and relayed rather than scanned in person, the MITM
persists. User education is the final control layer: verification must always happen in person or
via a real-time trusted channel.

---

#### Threat 2 — LAN Eavesdropping

**Attack description.**
An attacker on the same Wi-Fi network segment — a neighbour on a residential router, a co-worker
on a corporate WLAN, or a passive observer on a coffee-shop network — uses packet capture
(e.g., Wireshark) to record all QUIC UDP datagrams exchanged between devices.

**Why this matters.**
LAN-layer networks are not inherently trusted. Guest network bridging, ARP spoofing, and passive
monitoring are all feasible on shared network infrastructure. An attacker who can read transport-
layer traffic would otherwise be able to read all messages if they were transmitted in plaintext.

**Mitigation.**
All message payloads are AES-256-GCM encrypted by the Double Ratchet before being passed to the
QUIC transport layer. QUIC itself encrypts connection metadata including stream headers using
TLS 1.3, so stream boundaries and payload lengths are not exposed beyond the UDP datagram level.
The combination means a passive observer sees only UDP datagrams whose content they cannot
decrypt.

mDNS discovery (`_rimus-chat._udp.local`) announces device presence on the local network, but
announcements contain only the minimum information needed for connection setup (device ID,
transport endpoint). No message content, group membership, or key material is included in
discovery advertisements.

Group messages and pairwise messages share the same outer QUIC framing and are structurally
indistinguishable at the wire level.

**Residual risk.**
Traffic metadata is unavoidably visible: which IP addresses communicate, when, and the approximate
size of each datagram. An adversary with sufficient metadata can infer social relationships,
communication patterns, and group membership even without reading message content. No padding or
traffic shaping is currently implemented.

---

#### Threat 3 — Relay Server Compromise

**Attack description.**
An attacker gains full access to the relay server — its SQLite database, process memory, and
network stack. They can read every row in the database, inject forged relay frames into the offline
queue, observe all TLS session metadata, and modify the relay software to log all future traffic.

**Why this matters.**
The relay server is the central coordination point for multi-network communication and for offline
message queuing. It is a high-value target: compromising it provides access to the complete
communication graph of all relay-registered devices.

**Mitigation.**
The relay operates on an **opaque-forwarding model**: it stores and forwards encrypted peer frames
without any capability to decrypt them. Database rows contain device IDs, timestamps, and
ciphertext blobs. Even with full database access, an attacker cannot recover message plaintext
because message keys are derived from the Double Ratchet state held only on endpoint devices.

Device authentication uses Ed25519 challenge-response
(`apps/localmessenger_server/src/auth.rs`, `Authenticator::verify_response`): a random 32-byte
nonce is issued, the device signs it with its identity key, and the server verifies the signature
against the registered public key. The nonce is consumed on first use (`challenge.consumed = true`)
and has a configurable TTL. A compromised server can issue a new nonce and observe who signs it,
but cannot forge a valid signature from a legitimate device's private key.

An attacker who injects ciphertext blobs into the offline queue cannot produce valid AES-GCM
authentication tags without knowing the session keys. Any injected frame will be silently
discarded after MAC verification failure on the receiving device.

In-memory per-device rate limiting (`apps/localmessenger_server/src/rate_limit.rs`) bounds
the relay's utility as an amplification platform, though this is a defence-in-depth measure
rather than a primary security control.

**Residual risk.**
A compromised relay has access to the full communication graph: which device IDs are registered,
which are online, and which device is sending frames to which other device. It can also mount
availability attacks (see Threat 9). These metadata risks cannot be mitigated without
architectural changes such as mix networks or onion routing.

---

#### Threat 4 — Stale Epoch After Member Removal

**Attack description.**
Alice is removed from the group. She retains the sender keys and the group epoch state she held
before removal. If the remaining group members fail to rotate the epoch, Alice can continue to
decrypt future group messages because she still holds valid sender-key material for the current
epoch. She could also re-inject her own group messages, which would appear as coming from a valid
member of epoch N.

**Why this matters.**
Post-removal confidentiality is a fundamental property for any group messaging system. Without it,
removing a member from a group provides only a UI-level exclusion, not a cryptographic one.

**Mitigation.**
`GroupSession::rotate_for_member_removal` (in `crates/messaging/src/group.rs`) is called whenever
a participant is removed. This function:

1. Increments `epoch` by one.
2. Generates a fresh `LocalSenderKeyState` with new `chain_key_seed`, `signing_key`, and
   `distribution_id` for the local device.
3. Produces a `GroupEpochRotation` with
   `reason = GroupRotationReason::MemberRemoved { device_id }` and a new `membership` set that
   excludes the removed device.
4. Returns the new `GroupSenderKeyDistribution` to be distributed over pairwise sessions
   to the remaining participants only.

Because the removed member does not receive the new distribution, they cannot derive any message
keys for epoch N+1. Any group message they attempt to send using their old epoch-N sender chain
will be rejected by remaining members who have already transitioned to epoch N+1.

The implementation test `rotating_epoch_for_member_removal_blocks_removed_member` in `group.rs`
validates this property end-to-end.

**Residual risk.**
The removed member retains access to all messages they legitimately received during the previous
epoch. Encryption cannot retroactively conceal already-delivered content. The practical guidance
is to treat any message sent before a removal as potentially known to the removed member.

---

#### Threat 5 — Device Theft / Physical Access to SQLite DB

**Attack description.**
An attacker physically recovers a lost or stolen device — or clones its storage via USB, ADB
access, or a backup — and attempts to extract message history, device keys, or session state
from the SQLite database files.

**Why this matters.**
Local storage is the last line of defence for message confidentiality. If raw database files can
be read, all cryptographic protection at the transport layer is irrelevant.

**Mitigation.**
All rows in the SQLite database are encrypted by `AtRestCipher`
(`crates/storage/src/cipher.rs`) using AES-256-GCM with a fresh random 12-byte nonce per record
and namespace-scoped associated data
(`localmessenger/storage/aad/v1 || namespace || lookup_key`). The storage key is a 256-bit
`StorageKey` that is never stored in plaintext alongside the database. Row lookup keys are
SHA-256 hashes of `(domain_prefix || namespace || identifier)`, so no plaintext identifier
appears in any database index.

On device revocation, the relay CLI `disable-device` subcommand marks the device's record as
`disabled = true` in the server registry. Disabled devices are rejected during
`Authenticator::verify_response` before any frame can be received or queued. The remaining group
members should immediately call `GroupSession::rotate_for_device_compromise` to distribute new
sender-key material to only the uncompromised devices.

Intermediate plaintext buffers produced during decryption are explicitly zeroed via
`plaintext.zeroize()` in `AtRestCipher::decrypt`, reducing the window during which key material
sits in heap memory.

**Residual risk.**
If the attacker also recovers the `StorageKey` — from an OS keychain, memory dump, or unencrypted
backup — they can decrypt the entire SQLite database, including all stored message history and
identity key material. OS-level full-disk encryption (BitLocker, FileVault, LUKS) is strongly
recommended as a complementary control. No in-app mechanism can compensate for a fully
compromised file system.

---

#### Threat 6 — Forged or Replayed Invite Link

**Attack description.**
An attacker intercepts an invite link sent over an insecure channel (unencrypted email, SMS),
replays it after its intended single use, or constructs a link pointing to a malicious relay
server in order to harvest the joining device's identity public key and register it on an
adversary-controlled server.

**Why this matters.**
Invite links are the primary onboarding mechanism for relay-connected devices. A weakly
authenticated invite could allow an unauthorised device to join the relay, or could redirect
a user to connect their identity to an attacker-controlled service.

**Mitigation.**
Invite links are signed with **HMAC-SHA256** using the server's `invite_secret`
(`apps/localmessenger_server/src/invite.rs`, `InviteService::create_invite` /
`InviteService::join_with_invite`). The `verify_invite_link` function verifies the MAC before
any server-side action is performed.

The server additionally enforces:

- `invite.status == "active"` — deactivated invites are rejected.
- `now_unix_ms <= expires_at_unix_ms` — expired invites are rejected.
- `used_count < max_uses` — over-limit invites are rejected.

Each invite link embeds the relay server's TLS certificate in DER form. The desktop client pins
this certificate when connecting (`LOCALMESSENGER_SERVER_CERT_DER`), so a link that redirects to
a different server will fail TLS certificate pinning even if the hostname resolves correctly.

**Residual risk.**
An attacker who knows or can brute-force the `invite_secret` can forge valid HMAC signatures for
any invite payload. Operators must configure a strong, randomly generated secret and keep it
private. Short or guessable secrets (e.g., the default `changeme`) substantially weaken this
guarantee.

---

#### Threat 7 — Message Replay / Duplicate Injection

**Attack description.**
An attacker records a valid encrypted message frame from the network and re-injects it into the
same session later — either to cause the receiver to process the same message twice, to advance
the receiver's delivery-order state in an unintended direction, or to trigger a
double-acknowledgement that would mislead the sender into removing a message from the retry
queue prematurely.

**Why this matters.**
Replay attacks can produce visible message duplication for the user. In a stateful protocol, they
can also advance engine state in unintended ways, potentially revealing information about the
expected next sequence number to an active attacker.

**Mitigation.**
The `MessagingEngine` (`crates/messaging/src/engine.rs`) tracks incoming messages in two
structures:

- `incoming_order_index`: a bounded set of seen `message_id` strings (capped at
  `MAX_TRACKED_INCOMING_ORDERS` entries, pruned by `prune_incoming_history`).
- `incoming_message_orders`: a `BTreeMap` from `delivery_order` to `message_id` tracking the
  contiguous receive window.

`handle_incoming_message` calls `validate_identifier` on the incoming `message_id` and rejects
any frame whose `message_id` is already in `incoming_order_index`, or whose `delivery_order` is
below `next_expected_incoming_order`. Replayed ACK envelopes are detected by the same mechanism.

At the group layer, `RemoteSenderKeyState::ensure_message_fresh` independently checks
`message_id_index` (exact-match dedup) and `message_number_index` (order-based tracking). This
defence-in-depth means a replayed group frame is rejected even if it somehow bypassed the pairwise
engine.

The Double Ratchet itself provides a third layer: decrypting with an already-consumed message key
returns `CryptoError::ReplayOrDuplicateMessage` from `DoubleRatchet::decrypt`.

**Residual risk.**
The in-memory replay window is bounded by `MAX_TRACKED_INCOMING_ORDERS`. Very old frames that
fall outside the window are not tracked. More importantly, the pairwise replay window is not yet
persisted to SQLite across process restarts: a message captured before a restart and replayed
immediately after would pass the pairwise engine check (though the Double Ratchet would still
reject it if the ratchet state has advanced past the corresponding message key). Persisting the
replay state is listed as near-term work in Section 6.

---

#### Threat 8 — Key Material Exposure After App Crash

**Attack description.**
An unexpected process crash or OS-level kill leaves sensitive key material — session keys,
ratchet chain keys, storage keys, or plaintext buffers — in process memory, which an attacker
with local access to the device could recover from a core dump, swap partition, or a live memory
forensics tool.

**Why this matters.**
In-memory key material is an attractive target for forensic recovery because it bypasses all
encrypted-at-rest protections. Core dumps in particular may contain long-lived key material if
the process has been running for a while without being restartedl.

**Mitigation.**
The `zeroize` crate is used throughout the storage layer: in `AtRestCipher::encrypt` and
`AtRestCipher::decrypt`, the intermediate `plaintext` buffer is explicitly zeroed after use
(`plaintext.zeroize()`). The `StorageKey` type implements `Zeroize`. These calls reduce the
window during which key material sits in heap memory after it is no longer needed.

The durable pending queue means that a sudden crash does not lose in-flight messages; on restart
the queue is restored from the encrypted SQLite store rather than from a potentially exposed
in-memory snapshot.

**Residual risk.**
Core dumps, swap partitions, and live memory forensics can still recover key material from a
running or recently crashed process. The `zeroize` crate is designed to resist compiler
optimisation elision of zero-writes, but there is no hardware-backed secure enclave protecting
in-memory keys at present. OS-level mitigations — disabling core dumps, using `mlock` to prevent
paging, enabling encrypted swap — are recommended for high-sensitivity deployments and are outside
the application's direct control.

---

#### Threat 9 — Malicious Relay Blocking Delivery (Availability Attack)

**Attack description.**
A relay operator — or an attacker who has compromised the relay — selectively drops messages
destined for specific device IDs, delays their delivery beyond any practical session timeout, or
simply takes the relay offline. This denies the targeted device the ability to participate in
conversations routed through the relay.

**Why this matters.**
Selective availability attacks can be used to silently exclude a specific group member from
receiving messages, or to create the appearance of network problems while actually performing
targeted censorship. They are especially hard to detect because they leave no cryptographic
artefact on either end.

**Mitigation.**
The pairwise delivery engine's durable pending queue
(`SqliteStorage::upsert_pending_outbound`) ensures the sending device retries unacknowledged
messages indefinitely — across process restarts — until a delivery ACK is received. Messages are
not silently discarded on the sender's side.

The desktop client supports a two-entry transport order
(`LOCALMESSENGER_TRANSPORT_ORDER=server_relay,direct_lan`). When relay authentication or
connection fails, the client automatically falls back to direct QUIC over the LAN. For peers on
the same local network, a relay-based availability attack is entirely ineffective because the LAN
path bypasses the relay completely.

**Residual risk.**
If both the relay and direct-LAN paths are unavailable (devices on separate networks, relay down
or adversarial), message delivery is suspended until connectivity is restored. There is currently
no cryptographic delivery receipt, no forward-delivery proof, and no third-party audit trail to
detect selective suppression. Operators who require strong availability guarantees should run
their own relay on infrastructure they control and monitor independently.

---

#### Threat 10 — Browser Fallback Dev Path Used in Production

**Attack description.**
The `apps/tauri-client` frontend includes a browser fallback backend that allows the React UI
to be developed and tested without the full Tauri shell. This fallback uses mock data, in-memory
state, and no real transport or cryptography. If a developer accidentally builds and serves the
static `dist/` directory as a web application without the Tauri shell, end users would interact
with an interface that simulates encryption without performing any.

**Why this matters.**
The browser fallback bypasses the entire Rust command layer: no QUIC transport, no Double Ratchet
sessions, no SQLite encrypted storage, no real peer communication. Messages "sent" through the
fallback are never encrypted, transmitted, or received by real peers. A user relying on the
fallback for secure communication would receive no security whatsoever while believing they have
full protection.

**Mitigation.**
The browser fallback is clearly documented as a development-only path in both the codebase and
the README. The production application is always distributed as a Tauri bundle (`.exe`, `.dmg`,
`.AppImage`, `.deb`) that runs the full Tauri shell and Rust command layer. GitHub Actions CI/CD
publishes only the Tauri-bundled installer artefacts. The fallback backend contains no
cryptographic or transport logic and cannot connect to real peers or a real relay server.

IPC between the React frontend and the Rust backend is restricted to a set of registered Tauri
commands defined in `apps/tauri-client/src-tauri/src/commands.rs`; there is no path for the
frontend to bypass the Rust layer and call OS APIs directly.

**Residual risk.**
A developer who intentionally or accidentally serves the static `dist/` build to end users
outside the Tauri shell would provide a non-functional and entirely insecure experience. The
control is developer discipline and build pipeline enforcement rather than a cryptographic
mechanism. Ensure CI/CD artefact publishing is restricted to Tauri bundles only.

---

## 5. Implementation Status

This section maps each security property and threat mitigation to the specific crates and modules
that implement it, referencing real source files in the repository.

### 5.1 `crates/crypto` — Cryptographic Primitives

| Property | Source file | Key construct |
|----------|-------------|---------------|
| AES-256-GCM message encryption/decryption | `src/ratchet.rs` | `DoubleRatchet::encrypt` / `decrypt`; `encrypt_aead` / `decrypt_aead` using `aes_gcm::Aes256Gcm` |
| Double Ratchet forward secrecy | `src/ratchet.rs` | `DoubleRatchet::apply_remote_ratchet`; new X25519 key generated per ratchet step; old keys discarded |
| Forward secrecy state validation | `src/ratchet.rs` | `DoubleRatchet::state_snapshot` → `RatchetStateSnapshot`; compared in unit tests |
| Replay detection (ratchet layer) | `src/ratchet.rs` | `CryptoError::ReplayOrDuplicateMessage` returned when `message_number < receiving_chain.current_number()` |
| Out-of-order message key storage | `src/ratchet.rs` | `DoubleRatchet::skip_message_keys`; `skipped_message_keys: BTreeMap<SkippedMessageId, MessageKeyMaterial>`; bounded by `MAX_SKIP = 64` |
| X3DH session bootstrap | `src/x3dh.rs` | `x3dh_initiate` / `x3dh_respond` |
| Ed25519 identity keys | `src/identity.rs` | `IdentityKeyPair::generate`; `sign_message` / `signing_public` |
| HKDF-SHA256 key derivation | `src/kdf.rs` | `root_kdf`, `chain_kdf`; domain-separated via `localmessenger/` prefixes |

### 5.2 `crates/messaging` — Protocol Engine

| Property | Source file | Key construct |
|----------|-------------|---------------|
| Pairwise replay protection | `src/engine.rs` | `MessagingEngine::handle_incoming_message`; `incoming_order_index` + `incoming_message_orders` |
| Bounded replay window | `src/engine.rs` | `MAX_TRACKED_INCOMING_ORDERS`; `prune_incoming_history` keeps window finite |
| Durable pending queue export/restore | `src/engine.rs` | `export_pending_queue` → `PendingQueueSnapshot`; `restore_pending_queue` |
| Out-of-order delivery buffering | `src/engine.rs` | `buffered_incoming: BTreeMap`; messages held until contiguous delivery order is restored |
| Group replay protection | `src/group.rs` | `RemoteSenderKeyState::ensure_message_fresh`; `message_id_index` + `message_number_index` |
| Same-epoch sender-key replacement guard | `src/group.rs` | `GroupSession::import_sender_key`; conflicting distribution in same epoch returns a hard error |
| Epoch rotation on member removal | `src/group.rs` | `GroupSession::rotate_for_member_removal` |
| Epoch rotation on device compromise | `src/group.rs` | `GroupSession::rotate_for_device_compromise`; `GroupRotationReason::DeviceCompromised` |
| Group message Ed25519 signatures | `src/group.rs` | `sign_group_message` / `verify_group_message_signature`; covers `(group_id, epoch, distribution_id, message_id, message_number, ciphertext)` |
| X3DH + Double Ratchet secure session | `src/handshake.rs`, `src/session.rs` | `SecureSession` constructed from `HandshakeResult` |

### 5.3 `crates/storage` — Encrypted-at-Rest Persistence

| Property | Source file | Key construct |
|----------|-------------|---------------|
| AES-256-GCM at-rest encryption | `src/cipher.rs` | `AtRestCipher::encrypt` / `decrypt`; `OsRng`-generated 12-byte nonce per record |
| Namespace-scoped associated data | `src/cipher.rs` | `associated_data()` builds `localmessenger/storage/aad/v1 \|\| namespace \|\| lookup_key` |
| Hashed (opaque) row lookup keys | `src/store.rs` | `opaque_lookup_key()` applies SHA-256 with `localmessenger/storage/index/v1` domain prefix; plaintext IDs never stored in index |
| Durable pending outbound queue | `src/store.rs` | `SqliteStorage::upsert_pending_outbound`; `pending_outbound_for_peer`; `remove_pending_outbound` |
| Zeroize after decrypt | `src/cipher.rs` | `plaintext.zeroize()` immediately after `bincode::deserialize` |
| Encrypted local identity/prekey storage | `src/store.rs` | `store_local_device_secrets` / `local_device_secrets` in `local_device_secrets` table; same `AtRestCipher` |

### 5.4 `apps/localmessenger_server` — Relay Server

| Property | Source file | Key construct |
|----------|-------------|---------------|
| Ed25519 challenge-response auth | `src/auth.rs` | `Authenticator::verify_response`; nonce is single-use (`challenge.consumed = true`) and TTL-bounded |
| Auth rejection of disabled devices | `src/auth.rs` | `record.disabled` check before signature verification |
| HMAC-SHA256 signed invite links | `src/invite.rs` | `InviteService::create_invite` / `join_with_invite`; `encode_invite_link` / `verify_invite_link` |
| Invite expiry and use-count checks | `src/invite.rs` | `status`, `expires_at_unix_ms`, `used_count < max_uses` validated in `join_with_invite` |
| Opaque frame forwarding | `src/relay.rs` | Relay stores and forwards encrypted frames without parsing payload content |
| Device revocation | `src/registry.rs`, `src/main.rs` | `disable-device` CLI subcommand; sets `disabled = true` in registry; rejected by `Authenticator` |
| Per-device rate limiting | `src/rate_limit.rs` | In-memory per-device counters for relay frames, blob requests, blob bandwidth, and health checks |

### 5.5 `apps/tauri-client/src-tauri/src/state.rs` — Desktop Client

| Property | Source file | Key construct |
|----------|-------------|---------------|
| Durable pending queue restore on startup | `state.rs` | `ClientState::bootstrap` calls `SqliteStorage::pending_outbound_for_peer` and feeds `restore_pending_queue` into `MessagingEngine` |
| Device verification (QR + Safety Number) | `state.rs` | `ClientState::verify_device` calls real `crates/core` safety-number and QR validation; updates device trust state |
| Relay-to-LAN fallback | `state.rs` | `preferred_routes` field; `LOCALMESSENGER_TRANSPORT_ORDER=server_relay,direct_lan` |
| Encrypted storage binding | `state.rs` | `pending_store: SqliteStorage` field; all writes route through `AtRestCipher` |
| Relay + auth status surfacing | `state.rs` | `ClientSnapshot::server_status`, `auth_status`, `active_route` fields give users visibility into transport security |

---

## 6. Known Limitations and Future Work

### 6.1 No Formal Security Audit

The cryptographic protocol design and Rust implementation have not been formally reviewed by an
independent security auditor or cryptographer. The primitive libraries are industry-standard
audited implementations, but the protocol composition — X3DH feeding into Double Ratchet feeding
into the group sender-key layer — has only been reviewed internally. An independent audit is the
highest-priority security improvement for any deployment beyond a personal trusted group.

### 6.2 mDNS Discovery Metadata Not Encrypted

The mDNS TXT records advertised by `crates/discovery` contain device IDs and transport endpoints
in plaintext. Any host on the local network segment can enumerate Local Messenger devices, even
if they cannot decrypt any messages. This leaks group membership metadata on the LAN. A future
improvement could use encrypted discovery beacons or require out-of-band exchange of peer
endpoints rather than broadcast advertisement.

### 6.3 Sender-Key Fan-Out Is Currently Demo-Loopback

The current implementation of group sender-key distribution runs in-process (loopback) for
testing purposes rather than through a real network fan-out path. Each device holds the sender
keys for all participants in the unit test environment, but the mechanism that carries
`GroupSenderKeyDistribution` messages over live pairwise secure sessions to all group members has
not yet been wired to the networking layer. Until this is implemented, group messaging is not
end-to-end encrypted between real remote devices on different machines.

### 6.4 No Anti-Entropy Sync Between Own Devices

A user who runs Local Messenger on multiple devices (e.g., a laptop and a phone) has no mechanism
to synchronise message history between them. Each device has an independent ratchet state and
receives only frames addressed to it. There is no cross-device message history merge, no backup
mechanism, and no multi-device key linkage beyond what the relay's device registry provides.

### 6.5 No Certificate Transparency or Key Transparency

There is no append-only public log of device identity-key registrations, no mechanism for group
members to audit the history of key changes, and no third-party verifier that can confirm a
device's public key has not been silently replaced. Key transparency mechanisms (analogous to
WhatsApp's Key Transparency or CONIKS) would materially strengthen authentication guarantees but
are not yet planned.

### 6.6 Pairwise Replay State Not Durable Across Restarts

The pairwise replay-protection window (`incoming_order_index` in `MessagingEngine`) is an in-
memory data structure that is not currently persisted to SQLite. After an application restart,
the window is empty. A sufficiently motivated attacker who captures a message before a restart
and replays it immediately after could bypass pairwise engine replay detection — though the
Double Ratchet layer still rejects the replay if the ratchet state has advanced past the
corresponding message key. Persisting this state to the encrypted SQLite store is a near-term
improvement.

---

## 7. Security Contact

To report a security vulnerability in Local Messenger, please open a **GitHub Security Advisory**
at:

**<https://github.com/Rimus0cod/Local_sms/security/advisories/new>**

Please do **not** open a public issue for security vulnerabilities. Your report should include:

- A clear description of the vulnerability and the affected component.
- Step-by-step reproduction instructions.
- An assessment of the potential impact (confidentiality, integrity, availability).
- Any suggested mitigations or patches, if you have them.

The maintainers will acknowledge receipt within **72 hours** and provide an initial severity
assessment within **7 days**. Critical issues that affect message confidentiality or device
authentication will be prioritised for immediate patching and coordinated disclosure.