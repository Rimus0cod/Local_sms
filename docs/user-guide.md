# Local Messenger — User Guide

**Version:** 1.0.0
**Last updated:** 2024-04-01

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Installing the App](#2-installing-the-app)
3. [First Launch — Your Device Identity](#3-first-launch--your-device-identity)
4. [Setting Up a Direct Chat (Step by Step)](#4-setting-up-a-direct-chat-step-by-step)
5. [Device Verification](#5-device-verification)
6. [Group Chats](#6-group-chats)
7. [Setting Up a Relay Server](#7-setting-up-a-relay-server)
8. [Sending Files and Media](#8-sending-files-and-media)
9. [Working Offline / LAN-Only Mode](#9-working-offline--lan-only-mode)
10. [FAQ](#10-faq)
11. [Security Best Practices](#11-security-best-practices)

---

## 1. Introduction

### What Is Local Messenger?

Local Messenger is a **local-first, end-to-end encrypted** messaging application designed for
small trusted groups of up to 8 people. It is built on the same cryptographic foundations used
by Signal — X3DH for session setup, the Double Ratchet for per-message key rotation, and
AES-256-GCM for ciphertext protection — but is specifically optimised for groups whose members
know each other in real life and primarily communicate over a shared local network (Wi-Fi).

Key design principles:

- **No cloud account required.** Your identity is a cryptographic key pair generated entirely on
  your device. No email address, phone number, or server registration is needed to start chatting
  on the local network.
- **No central server for message content.** Messages travel directly between devices over QUIC
  on the LAN. When a relay server is used (for cross-network communication), it stores and
  forwards only opaque encrypted blobs that it cannot decrypt.
- **Local-first storage.** Your message history lives in an AES-256-GCM encrypted SQLite
  database on your own device. Nothing is stored in the cloud.
- **Small trusted groups.** The maximum group size is 8 members. This is a deliberate design
  choice: it keeps the key-distribution model simple and keeps the social trust model honest.

### When to Use Local Messenger

Local Messenger is a good fit when:

- You want private, encrypted messaging with close colleagues, family members, or a small team
  who share the same Wi-Fi network most of the time.
- You want to control your own relay infrastructure and never hand message metadata to a third
  party.
- You prefer a lightweight desktop application without cloud sign-up friction.

### When NOT to Use Local Messenger

> **Security disclaimer.** Please read this before using the app for sensitive communications.

Local Messenger is **not** appropriate when:

- You need to communicate with people you have never met or cannot verify in person or via a
  trusted real-time call. The security guarantees depend on completing device verification.
- You need anonymity. All group members can see each other's device identifiers and presence
  status.
- You need a formal security audit. This software has not yet been independently audited.
- You need guaranteed message delivery. A relay server or LAN connection must be available;
  there is no fallback to SMS or push notifications.
- Your threat model includes a fully compromised operating system. Application-layer encryption
  cannot protect against OS-level attacks.

---

## 2. Installing the App

### Pre-built Installers

Pre-built installers for Windows, macOS, and Linux are published with every tagged release:

**https://github.com/Rimus0cod/Local_sms/releases/latest**

#### Windows

1. Download `LocalMessenger_x.x.x_x64-setup.exe` (NSIS installer) **or**
   `LocalMessenger_x.x.x_x64_en-US.msi` (MSI package).
2. Run the installer and follow the on-screen prompts.
3. Launch **Local Messenger** from the Start Menu or the Desktop shortcut.

#### macOS

1. Download `LocalMessenger_x.x.x_x64.dmg` (Intel) or `_aarch64.dmg` (Apple Silicon).
2. Open the `.dmg` file and drag **Local Messenger** into your **Applications** folder.
3. On first launch, macOS may show a Gatekeeper warning. Open
   **System Settings → Privacy & Security** and click **Open Anyway**.

#### Linux

AppImage (works on most distributions, no installation required):

```
chmod +x LocalMessenger_x.x.x_amd64.AppImage
./LocalMessenger_x.x.x_amd64.AppImage
```

Debian / Ubuntu package:

```
sudo dpkg -i localmessenger_x.x.x_amd64.deb
```

### System Requirements

| Platform | Minimum requirement |
|----------|-------------------|
| Windows  | Windows 10 64-bit or later; WebView2 runtime (installed automatically by the NSIS installer) |
| macOS    | macOS 11 Big Sur or later; Apple Silicon or Intel x86-64 |
| Linux    | glibc 2.31+; WebKit2GTK 4.1; a running D-Bus session |

For building from source, see the **Build from Source** section in `README.md`.

---

## 3. First Launch — Your Device Identity

### What Happens on First Launch

The very first time you open Local Messenger, the application automatically generates your
**device identity**: an Ed25519 signing key pair that uniquely identifies your device to all
other Local Messenger users you will ever chat with. This process:

- Happens entirely on your device — no network connection is required.
- Takes less than a second.
- Requires no input from you.
- Is completely automatic — you will land directly on the main window with an empty chat list.

> **Important:** Your identity key is generated fresh on each device and each new installation.
> If you reinstall the app, a new identity is created. Other users will need to re-verify you
> after a reinstall.

### Where Keys Are Stored

All key material — your Ed25519 identity key, your X25519 ratchet keys, and your one-time
prekeys — is stored in an **AES-256-GCM encrypted SQLite database** inside the application's
data directory:

| Platform | Default data directory |
|----------|----------------------|
| Windows  | `%APPDATA%\Local Messenger\` |
| macOS    | `~/Library/Application Support/Local Messenger/` |
| Linux    | `~/.local/share/localmessenger/` |

The database file is `localmessenger.db`. **Do not copy, move, or manually edit this file.**
Your private keys never leave this encrypted database and are never transmitted over the network
in plaintext.

### Your Device Profile

After the first launch, you can set your display name under **Settings → Profile**. This name
is broadcast to peers you discover on the LAN and is stored alongside your device ID in the
relay server's registry when you join a relay. It is purely cosmetic and has no effect on
cryptographic identity.

---

## 4. Setting Up a Direct Chat (Step by Step)

A direct chat is a one-to-one encrypted conversation between two devices on the same local
network. No relay server is needed.

### Step 1 — Be on the Same Wi-Fi

Both devices must be connected to the same Wi-Fi network or the same local network segment.
Devices on different VLANs, or separated by a router with multicast isolation, may not be
able to discover each other automatically (see Section 9 for troubleshooting).

### Step 2 — Both People Launch Local Messenger

The other person must have Local Messenger installed and running on their device. The app
advertises its presence on the LAN using mDNS (the `_rimus-chat._udp.local` service). There
is nothing to configure — the advertisement starts automatically when the app opens.

### Step 3 — Open Settings → Network and Peers

Navigate to **Settings → Network and Peers** (or click the **Peers** icon in the sidebar).
After a few seconds, you should see the other person's device appear in the **Discovered Peers**
list with their display name, device name, and IP address.

If the peer does not appear, see the FAQ at the end of this guide.

### Step 4 — Click "Start Chat" Next to Their Device

Click the **Start chat** button next to the peer's device entry. The app initiates an
**X3DH cryptographic handshake** over a direct QUIC connection to the other device. This
handshake:

1. Exchanges identity and prekey material.
2. Derives a shared root key that seeds the Double Ratchet.
3. Establishes a verified session binding that ties the session to both devices' identity keys.

The new chat thread appears in the left-hand chat list. The peer's device name is shown in the
thread header. The security label at the top of the chat window will show
**"Forward secrecy active"** once the first message has been exchanged.

### Step 5 — Verify the Device (IMPORTANT)

Before sharing any sensitive information, complete device verification (see Section 5). The
chat is encrypted from the moment the handshake completes, but verification confirms that you
are talking to the right person and not to an attacker who intercepted the handshake.

---

## 5. Device Verification

### Why Verification Matters

When two devices perform a cryptographic handshake for the first time, there is a window during
which an attacker on the same network could substitute their own keys for yours — a
**man-in-the-middle (MITM) attack**. The attacker would see all messages in both directions
until the attack is detected.

Verification closes this window by comparing a short fingerprint derived from both devices'
public identity keys. If the fingerprints match on both sides, no key substitution occurred.
**Complete verification before sharing anything you would not want an eavesdropper to see.**

### Method A — QR Code (Recommended for In-Person Verification)

This method is the easiest when both users are physically in the same room.

1. Open the chat with the other person and click **Verify Device** (or go to
   **Settings → Verification** and select their device).
2. Your device displays a QR code encoding your identity public key.
3. The other person opens the same verification screen on their device and uses the QR scanner
   to scan your QR code.
4. If the keys match, the app shows **"Verification successful"** on both sides.
5. Repeat in the other direction: they show their QR code; you scan it.

Both sides must complete verification for the "Verified" badge to appear on the chat thread.

### Method B — Safety Number (For Remote Verification)

Use this method when you cannot meet in person but can make a voice or video call.

1. Open **Settings → Verification** and select the other person's device.
2. Both of you independently tap **Show Safety Number**.
3. The app displays a safety number consisting of several groups of digits.
4. Read your safety number aloud over the call while the other person follows along on their
   screen. Then they read theirs while you follow.
5. If both safety numbers match **exactly** on both sides, the session is authentic.
6. Tap **Mark as Verified** once both numbers have been confirmed.

> **Warning:** Do not compare safety numbers by text message, email, or any channel that might
> itself be intercepted. Use a real-time voice or video call.

### What "Verified" Status Means

A **Verified** badge on a chat thread or in the peer list means:

- Both devices have completed out-of-band key comparison.
- The Ed25519 identity keys used in the current session match the keys that were verified.
- No key substitution occurred at the time of verification.

If the other person reinstalls the app or gets a new device, their identity key changes and the
verification status resets to **Pending**. You will need to verify again before trusting the
new session.

---

## 6. Group Chats

### How Groups Are Formed

Local Messenger uses a **Signal-style sender-key** architecture for group messaging. Each device
in a group holds its own symmetric sender-key chain. Group messages are encrypted once by the
sender under their own chain key and delivered to each recipient as a separate pairwise frame,
so the sender encrypts exactly once regardless of group size.

In the current release, a group bootstraps automatically with all LAN-reachable verified peers.
Navigate to **Chats → New Group** to create a group from your verified contact list. The first
epoch (epoch 0) is established when all members exchange sender-key distribution messages.

### Group Epochs and Why They Matter

A **group epoch** is a numbered generation of the group's cryptographic state. Every sender-key
distribution, every message, and every membership event is tagged with an epoch number.

- Epoch 0 is the initial state when the group is first formed.
- The epoch increments every time membership changes or a manual forward-secrecy refresh is
  triggered.
- After an epoch rotation, the old sender keys are discarded and new keys are generated and
  distributed only to the members of the new epoch.

This design means that a member who leaves — or is removed — cannot decrypt messages sent after
the epoch rotation, even if they captured every message up to that point.

### Adding Members

Adding a new member to an existing group triggers an **epoch rotation**:

1. The group admin generates a new epoch number.
2. New sender-key distributions are generated for all members including the new one.
3. The distributions are delivered over pairwise encrypted sessions.
4. Only after receiving the new distribution can a device send or receive group messages for the
   new epoch.

The new member can participate fully in the new epoch but cannot decrypt any message from
previous epochs.

### Removing Members

Removing a member is the most security-sensitive operation in group management:

1. Any remaining member can initiate a removal, but the epoch rotation must be acknowledged by
   all remaining members before the removed member is truly excluded.
2. The group automatically triggers `rotate_for_member_removal`, which increments the epoch and
   generates new sender-key material.
3. The new distributions are sent only to the remaining members.
4. The removed member retains all messages they legitimately received before removal.

> After removing a member, treat all messages from previous epochs as potentially known to the
> removed party.

### Group Media

Sending images, audio, or files in a group chat requires a relay server when group members are
on different networks. On a LAN where all members are reachable directly, media transfers use
direct QUIC connections. See Section 7 for relay setup and Section 8 for media details.

---

## 7. Setting Up a Relay Server

### Why You Need a Relay Server

The relay server enables communication between devices on **different networks** — for example,
when one member is at home and another is at the office, or when group members are spread across
different cities. Without a relay, the app works only when all devices are on the same Wi-Fi.

The relay also acts as a **store-and-forward queue**: if a device is offline when a message is
sent, the relay holds the encrypted frame and delivers it when the device reconnects.

> The relay server never sees the plaintext of any message. It stores and forwards only opaque
> AES-256-GCM encrypted blobs.

### Quick Docker Setup (Recommended — Under 5 Minutes)

See `docker/README.md` for the full reference. The short version:

**Step 1 — Copy the environment template**

```
cp docker/.env.example docker/.env
```

Open `docker/.env` and set at minimum:

```
SERVER_NAME=relay.example.com
INVITE_SECRET=a-long-randomly-generated-secret
```

`SERVER_NAME` should be the public hostname or IP address that clients will use to connect.
`INVITE_SECRET` signs all invite links — keep it private and use a strong random value.

**Step 2 — Build and start the relay**

```
cd docker
docker compose up -d --build
```

The relay starts on UDP port 7443. On first boot the entrypoint generates a self-signed TLS
certificate and stores it in the `relay-data` Docker volume.

**Step 3 — Create an invite link**

```
docker compose exec relay localmessenger_server create-invite \
    --label "Home relay" \
    --max-uses 8 \
    --expires-in 72h
```

The command prints a `localmessenger://join/…` URL. Copy it.

**Step 4 — Share the invite link**

Send the invite URL to each person who should join your relay. They will paste it into the
desktop client as described below.

**Step 5 — Monitor the relay**

```
docker compose exec relay localmessenger_server list-devices \
    --db /data/relay.db
```

### Manual Setup

For a non-Docker deployment (e.g., a VPS or bare-metal server), see `docs/server-relay.md` for
the full CLI reference, systemd service example, and environment variable descriptions.

The key CLI commands are:

```
# Start the server
cargo run -p localmessenger_server -- serve \
  --bind 0.0.0.0:7443 \
  --server-name relay.example.com \
  --cert /path/relay-cert.der \
  --key  /path/relay-key.der \
  --db   /path/relay.db \
  --invite-secret YOUR_SECRET

# Generate a self-signed certificate
cargo run -p localmessenger_server -- gen-cert \
  --server-name relay.example.com \
  --cert-out /path/relay-cert.der \
  --key-out  /path/relay-key.der

# Create an invite link
cargo run -p localmessenger_server -- create-invite \
  --db /path/relay.db \
  --invite-secret YOUR_SECRET \
  --label "Alice's invite" \
  --server-addr 203.0.113.10:7443 \
  --server-name relay.example.com \
  --cert /path/relay-cert.der \
  --ttl-seconds 86400 \
  --max-uses 1
```

### Joining via Invite Link

Once someone has sent you an invite link:

1. Open Local Messenger on your desktop.
2. Go to **Settings → Relay Server → Join with Invite Link**.
3. Paste the `localmessenger://join/…` URL into the input field.
4. Click **Join**. The app verifies the HMAC signature on the link, connects to the relay
   server, and completes the Ed25519 challenge-response authentication.
5. The relay status in the toolbar will change to **Connected** once authentication succeeds.

### Revoking Access

To prevent a device from receiving future relay messages, use the relay CLI:

```
cargo run -p localmessenger_server -- disable-device \
  --db /path/relay.db \
  --device-id alice-phone
```

After revoking, rotate the group epoch immediately so the removed device cannot decrypt future
group messages (see Section 6).

---

## 8. Sending Files and Media

### Supported File Types

Local Messenger supports sending:

- **Images** — JPEG, PNG, GIF, WebP; rendered inline as photo previews in the chat window.
- **Audio / Voice Notes** — record a voice note directly in the app or attach an audio file;
  played back inline with a waveform control.
- **PDF Documents** — rendered as a document preview with page thumbnails.
- **Any file** — arbitrary binary files can be attached to any message.

### Size Limits and Routing

| File size | Transfer route |
|-----------|---------------|
| Up to 5 MB | Encrypted blob stored on the relay server (`server_blob_store`); recipient downloads on demand |
| Over 5 MB | Direct QUIC P2P transfer (`p2p_quic_direct`); both devices must be reachable from each other |

Files over 5 MB require a direct connection between the two devices. If the devices are behind
NAT without a direct path, large file transfers will fail. In that case, split the file or
compress it below 5 MB before sending.

### How Encryption Works

When you attach a file:

1. The app generates a random AES-256-GCM encryption key for the blob.
2. The blob is encrypted on your device before any network transfer begins.
3. For relay-routed blobs: the encrypted ciphertext is uploaded to the relay server in chunks.
   The relay stores only the ciphertext — it cannot decrypt it.
4. The encryption key is included in the message envelope, which is itself end-to-end encrypted
   for each recipient.
5. When the recipient opens the attachment, the app downloads the encrypted blob from the relay
   and decrypts it locally using the key from the message envelope.

The relay server never possesses a plaintext copy of any attachment.

### Upload Progress

A progress bar appears in the chat bubble while a file is uploading. Tauri events stream
upload progress from the Rust backend to the React frontend in real time. If the upload is
interrupted, the app retries from the last successfully uploaded chunk when connectivity
is restored.

---

## 9. Working Offline / LAN-Only Mode

### LAN-Only Operation

Local Messenger works completely without internet access as long as all group members are
connected to the same local Wi-Fi network. In LAN-only mode:

- Peer discovery uses **mDNS** (`_rimus-chat._udp.local`). No DNS server, no internet, no relay
  is required.
- All messages travel directly over QUIC between devices on the LAN.
- File transfers up to 5 MB use direct QUIC P2P; there is no relay to upload to.
- Group messages are delivered directly to each member device on the LAN.

This is the default and most private operating mode. The relay server is only contacted when
a peer is not discoverable on the LAN.

### Automatic Peer Discovery

When the app starts, the discovery service broadcasts a presence announcement on the LAN every
few seconds. Peers appear in **Settings → Network and Peers** automatically — no IP addresses
to type, no hostnames to remember. If a peer goes offline, their entry in the peer list is
marked **Offline** after a short expiry window and their in-flight messages are moved to the
durable retry queue.

### Offline Message Queue

If you send a message and the recipient is currently offline (or temporarily unreachable):

1. The message is encrypted and stored in an encrypted `pending_outbound_queue` SQLite table
   on your device.
2. The app retries delivery in the background whenever the peer reconnects.
3. **The pending queue survives app restarts.** Even if you close and reopen Local Messenger,
   the queued messages are restored and delivery resumes automatically the next time the
   peer is reachable.
4. Once the peer acknowledges receipt, the message is removed from the queue and the delivery
   status in the chat window updates to **Delivered**.

### Transport Priority

The app tries transport routes in the order configured by
`LOCALMESSENGER_TRANSPORT_ORDER`. The default is:

```
server_relay,direct_lan
```

This means: try the relay server first; if the relay is unavailable or authentication fails,
fall back to direct LAN QUIC. You can reverse the order to prefer LAN and only use the relay
as a fallback, or set it to `direct_lan` only for a purely local deployment with no relay.

---

## 10. FAQ

---

**Q: I don't see the other person's device in the peer list.**

A: Check the following in order:

1. Both devices must be on the **same Wi-Fi network or LAN segment**. Devices on separate VLANs,
   or separated by a router with multicast isolation or AP isolation enabled, cannot discover
   each other via mDNS. Ask your network administrator to enable multicast forwarding between
   the segments, or use a relay server instead.
2. Both instances of Local Messenger must be **running** on both devices. The discovery beacon
   is only active while the app is open.
3. Click **Refresh** in **Settings → Network and Peers**. Discovery sometimes takes 5–10 seconds.
4. Check your OS firewall. Local Messenger uses UDP for mDNS (port 5353) and QUIC (default port
   range). Allow these through any host-based firewall.
5. If none of the above work, set up a relay server (Section 7) and use an invite link to connect
   across networks.

---

**Q: Can I use this over the internet without a relay?**

A: Not reliably. The LAN discovery mechanism (mDNS) only works on the local network segment.
Without a relay, the app has no way to locate a peer on a different network or establish a
connection through NAT. You need a relay server for cross-network communication. The relay is
lightweight and can run on any VPS for a few dollars per month. See Section 7.

---

**Q: What happens if I lose my phone?**

A: Take the following steps immediately:

1. On the relay server, run `disable-device --device-id <your-phone-id>` to revoke the lost
   device's access to the relay. It will no longer be able to authenticate or receive messages.
2. Ask a remaining group member to trigger a group epoch rotation
   (`rotate_for_device_compromise`). This generates new sender keys and distributes them only
   to devices that were not revoked.
3. On your new device, reinstall Local Messenger, generate a new identity, and join the relay
   with a new invite link.
4. Re-verify your identity with all group members before resuming sensitive conversations.

Messages already delivered to the lost device before revocation are still at risk if the device
storage key is compromised. OS-level full-disk encryption (enabled by default on modern iOS and
Android, and configurable on macOS and Linux) limits this risk.

---

**Q: How do I remove someone from a group?**

A: Open the group chat, go to **Group Settings → Members**, and click **Remove** next to the
member's device. The app will:

1. Remove the member from the membership set.
2. Automatically trigger an epoch rotation (`rotate_for_member_removal`).
3. Distribute new sender-key material to the remaining members only.

After the rotation, the removed member cannot decrypt new group messages. The removed member's
last known messages to the group (before removal) remain visible in the chat history for the
remaining members.

---

**Q: Is my message history backed up?**

A: No. Local Messenger does not back up your message history to any cloud service. Your history
is stored solely in the encrypted SQLite database on your device. If you lose the device or
uninstall the app without manually copying the database, the history is gone. An encrypted
export feature is planned for a future release. Until then, manually back up the database file
from the data directory listed in Section 3.

---

**Q: Can the relay operator read my messages?**

A: No. The relay server stores and forwards only AES-256-GCM encrypted blobs. The encryption
key for each message is derived from the Double Ratchet session between the two endpoints and
is never sent to the relay. Even if the relay operator has full access to their database and
server memory, they cannot decrypt any message content or attachment.

What the relay operator *can* see: which device IDs are registered, when they connect and
disconnect, and which device is sending frames to which other device. They cannot read the
content of those frames. If you run your own relay, you control this metadata entirely.

---

**Q: What does "Verification required" mean?**

A: It means the chat session has been established cryptographically (messages are encrypted),
but you have not yet completed out-of-band verification to confirm you are talking to the
correct person. Until you verify, there is a theoretical risk that a man-in-the-middle attack
occurred during the initial handshake.

To clear this warning, complete QR code or safety-number verification as described in Section 5.
Do not share sensitive information until verification is complete.

---

**Q: The app says "Forward secrecy active" — what does that mean?**

A: It means the Double Ratchet is running and has advanced at least one ratchet step. Each
message advances the symmetric chain key, and each received message triggers a new DH ratchet
step that generates fresh key material. Crucially:

- Keys used to encrypt past messages are discarded and cannot be recovered.
- If an attacker were to steal the current session state, they could not decrypt any message
  that was encrypted before the most recent ratchet advance.

"Forward secrecy active" is the normal, expected status for any live chat that has exchanged
at least one message in each direction.

---

**Q: I got a "Group sender key epoch" error. What should I do?**

A: This error means the group received a sender-key distribution for an epoch that conflicts
with the current state — most commonly because a member attempted to replace an existing
sender key within the same epoch, which is treated as a protocol error.

Steps to resolve:

1. All group members should close and reopen the app to trigger a fresh state sync.
2. If the error persists, a group admin should manually trigger an epoch rotation via
   **Group Settings → Rotate Keys**.
3. If one specific device is consistently causing the error, it may indicate a corrupted local
   state on that device. That device should leave the group, reinstall the app, generate a new
   identity, and rejoin via invite.

---

## 11. Security Best Practices

Follow these practices to get the most security out of Local Messenger:

**Always verify devices before sharing sensitive information.**
Completing QR or safety-number verification before trusting a chat is the single most impactful
security action you can take. An unverified chat is encrypted but could theoretically be
intercepted. A verified chat cannot be MITM'd without the safety number visibly changing.

**Use a relay server you control or trust.**
A relay you run yourself gives you full visibility into connection metadata and eliminates the
risk of a third-party relay operator logging your communication graph. If you must use someone
else's relay, verify that the relay certificate in your invite link is the one you expect before
joining.

**Rotate invite link secrets regularly.**
The `INVITE_SECRET` on your relay server signs all invite links. Rotate it periodically and
revoke old invite links after their maximum use count has been reached. Do not share invite
links over channels that might be monitored.

**Revoke lost or stolen devices immediately.**
Run `disable-device` on the relay and trigger a group epoch rotation before the lost device can
receive any further messages. The faster you act, the smaller the window of potential exposure.

**Keep the app and OS updated.**
Security patches for the underlying cryptographic libraries and the Tauri shell are delivered
via the app's built-in updater. Check **Settings → About → Check for Updates** regularly, or
enable automatic update checks.

**Enable full-disk encryption on all devices.**
The app encrypts the SQLite database at the application layer, but the storage key must itself
be protected. OS-level full-disk encryption (BitLocker on Windows, FileVault on macOS, LUKS on
Linux) ensures that the database — and the storage key — cannot be read if the device is stolen
while powered off.

**Verify safety numbers after any re-install or device change.**
Any time a contact installs the app fresh, their identity key changes and their verification
status resets to Pending. Always re-verify before resuming sensitive conversations.

**Do not use the browser fallback for real conversations.**
The React frontend can be run in a browser without the Tauri shell for UI development purposes.
This mode has no real encryption or transport. Production use must always be through the
installed Tauri application, never through a web browser.