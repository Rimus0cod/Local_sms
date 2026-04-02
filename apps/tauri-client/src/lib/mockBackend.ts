import type {
  ChatThreadView,
  ClientSnapshot,
  VerificationAction,
} from "../types";

function createInitialSnapshot(): ClientSnapshot {
  return {
    transportStatus: {
      discoveryMode: "mDNS peer discovery",
      transportMode: "QUIC transport with relay fallback routing",
      cryptoMode: "X3DH bootstrap + Double Ratchet",
      storageMode: "Encrypted SQLite at rest",
      serverStatus: "disabled",
      authStatus: "disabled",
      activeRoute: "direct_lan",
    },
    serverStatus: "disabled",
    authStatus: "disabled",
    activeRoute: "direct_lan",
    notifications: {
      trayLabel: "1 unread",
      unreadCount: 1,
      lastEvent: "Bob sent a new message",
    },
    localProfile: {
      displayName: "Rimus",
      activeDeviceName: "Rimus Laptop",
      activeDeviceId: "rimus-laptop",
      trustedDeviceCount: 2,
      totalDeviceCount: 2,
    },
    chats: [
      {
        id: "chat-bob",
        title: "Bob",
        summary: "LAN sync window looks clean. Ready for file relay tests.",
        presenceLabel: "online in local Wi-Fi",
        presenceState: "online",
        unreadCount: 1,
        securityLabel: "Verified device pair",
        kind: "direct",
        participants: ["Rimus", "Bob"],
        messages: [
          {
            id: "m-1",
            author: "Bob",
            body: "I am back on the office Wi-Fi. QUIC path is stable now.",
            timestampLabel: "09:18",
            direction: "inbound",
            deliveryState: "delivered",
            forwardedFrom: null,
            replyPreview: null,
            reactions: ["ack"],
            attachments: [],
          },
          {
            id: "m-2",
            author: "Rimus",
            body: "Good. I want to push the sender-key rotation UI next.",
            timestampLabel: "09:22",
            direction: "outbound",
            deliveryState: "seen",
            forwardedFrom: null,
            replyPreview:
              "I am back on the office Wi-Fi. QUIC path is stable now.",
            reactions: [],
            attachments: [],
          },
          {
            id: "m-3",
            author: "Bob",
            body: "LAN sync window looks clean. Ready for file relay tests.",
            timestampLabel: "09:25",
            direction: "inbound",
            deliveryState: "delivered",
            forwardedFrom: null,
            replyPreview: null,
            reactions: ["+1"],
            attachments: [],
          },
        ],
      },
      {
        id: "chat-lan-crew",
        title: "LAN Crew",
        summary: "Epoch 4 is active after Carol workstation rejoin.",
        presenceLabel: "3 of 4 peers reachable",
        presenceState: "reconnecting",
        unreadCount: 0,
        securityLabel: "Group sender key epoch 4",
        kind: "group",
        participants: ["Rimus", "Bob", "Carol", "Daria"],
        messages: [
          {
            id: "g-1",
            author: "System",
            body: "Group sender-key epoch rotated after Carol workstation rejoin.",
            timestampLabel: "08:41",
            direction: "system",
            deliveryState: "delivered",
            forwardedFrom: null,
            replyPreview: null,
            reactions: [],
            attachments: [],
          },
          {
            id: "g-2",
            author: "Carol",
            body: "My workstation is reachable again. Discovery TTL looks normal.",
            timestampLabel: "08:45",
            direction: "inbound",
            deliveryState: "delivered",
            forwardedFrom: null,
            replyPreview: null,
            reactions: [],
            attachments: [],
          },
        ],
      },
      {
        id: "chat-carol",
        title: "Carol",
        summary: "Need to verify the tablet before enabling attachments.",
        presenceLabel: "reconnecting",
        presenceState: "reconnecting",
        unreadCount: 0,
        securityLabel: "One pending device",
        kind: "direct",
        participants: ["Rimus", "Carol"],
        messages: [
          {
            id: "c-1",
            author: "Carol",
            body: "Need to verify the tablet before enabling attachments.",
            timestampLabel: "Yesterday",
            direction: "inbound",
            deliveryState: "delivered",
            forwardedFrom: "LAN Crew",
            replyPreview: null,
            reactions: [],
            attachments: [
              {
                id: "mock-photo-1",
                fileName: "verification-board.jpg",
                mimeType: "image/svg+xml",
                sizeLabel: "18 KB",
                transferRoute: "server_blob_store",
                statusLabel: "encrypted relay blob cached",
                previewDataUrl: samplePhotoDataUrl(),
                blobId: "blob-mock-photo-1",
                uploadProgress: 1.0,
              },
              {
                id: "mock-voice-1",
                fileName: "voice-note.wav",
                mimeType: "audio/wav",
                sizeLabel: "12 KB",
                transferRoute: "server_blob_store",
                statusLabel: "voice note ready",
                previewDataUrl: sampleVoiceDataUrl(),
                blobId: "blob-mock-voice-1",
                uploadProgress: 1.0,
              },
              {
                id: "mock-pdf-1",
                fileName: "relay-hardening-brief.pdf",
                mimeType: "application/pdf",
                sizeLabel: "14 KB",
                transferRoute: "server_blob_store",
                statusLabel: "document preview ready",
                previewDataUrl: samplePdfDataUrl(),
                blobId: "blob-mock-pdf-1",
                uploadProgress: 1.0,
              },
            ],
          },
        ],
      },
    ],
    peers: [
      {
        memberId: "bob",
        deviceId: "bob-phone",
        deviceName: "Bob Phone",
        endpoint: "192.168.1.23:46011",
        hostname: "bob-phone.local",
        capabilities: ["messaging-v1", "presence-v1"],
        state: "live",
        trustLabel: "verified",
        lastSeenLabel: "seen just now",
      },
      {
        memberId: "carol",
        deviceId: "carol-workstation",
        deviceName: "Carol Workstation",
        endpoint: "192.168.1.31:46012",
        hostname: "carol-workstation.local",
        capabilities: ["messaging-v1", "files-v1", "presence-v1"],
        state: "reconnecting",
        trustLabel: "verified",
        lastSeenLabel: "seen 20s ago",
      },
      {
        memberId: "daria",
        deviceId: "daria-laptop",
        deviceName: "Daria Laptop",
        endpoint: "192.168.1.44:46018",
        hostname: "daria-laptop.local",
        capabilities: ["messaging-v1"],
        state: "dormant",
        trustLabel: "pending",
        lastSeenLabel: "seen 3m ago",
      },
    ],
    verification: {
      trustedDeviceCount: 2,
      pendingDeviceCount: 2,
      devices: [
        {
          memberId: "bob",
          memberName: "Bob",
          deviceId: "bob-phone",
          deviceName: "Bob Phone",
          state: "verified",
          method: "safety_number",
          safetyNumber:
            "55087 07044 25215 21399 77393 93678 68190 50758 87312 17825",
          qrPayloadHex: "01000000626f622d70686f6e65",
        },
        {
          memberId: "bob",
          memberName: "Bob",
          deviceId: "bob-tablet",
          deviceName: "Bob Tablet",
          state: "pending",
          method: null,
          safetyNumber:
            "19300 81942 01713 82410 27760 12554 68282 70991 71828 10412",
          qrPayloadHex: "01000000626f622d7461626c6574",
        },
        {
          memberId: "carol",
          memberName: "Carol",
          deviceId: "carol-workstation",
          deviceName: "Carol Workstation",
          state: "verified",
          method: "qr_code",
          safetyNumber:
            "64011 03774 18811 11995 20341 55078 55081 18820 30861 29014",
          qrPayloadHex: "010000006361726f6c2d776f726b73746174696f6e",
        },
        {
          memberId: "daria",
          memberName: "Daria",
          deviceId: "daria-laptop",
          deviceName: "Daria Laptop",
          state: "pending",
          method: null,
          safetyNumber:
            "02771 90337 61288 19817 77215 66392 31004 11889 60019 50808",
          qrPayloadHex: "0100000064617269612d6c6170746f70",
        },
      ],
    },
    onboarding: {
      statusLabel: "Paste an invite link to join a relay.",
      invitePreview: null,
      contactInvitePreview: null,
    },
    updater: {
      currentVersion: "0.1.0",
      channel: "stable",
      statusLabel:
        "Updater artifacts can be produced during bundling. Runtime auto-install stays disabled in the mock shell.",
      lastCheckedLabel: "never",
      canAutoUpdate: false,
      feedUrl: null,
    },
  };
}

let snapshot = createInitialSnapshot();
let messageCounter = 100;
let refreshCounter = 0;

function cloneSnapshot(): ClientSnapshot {
  return JSON.parse(JSON.stringify(snapshot)) as ClientSnapshot;
}

export async function loadSnapshot(): Promise<ClientSnapshot> {
  return cloneSnapshot();
}

export async function refreshPeers(): Promise<ClientSnapshot> {
  refreshCounter += 1;
  snapshot.peers = snapshot.peers.map((peer, index) => {
    if (index === 0) {
      return {
        ...peer,
        state: "live",
        lastSeenLabel: "seen just now",
      };
    }

    if (index === 1) {
      return {
        ...peer,
        state: refreshCounter % 2 === 0 ? "live" : "reconnecting",
        lastSeenLabel:
          refreshCounter % 2 === 0 ? "seen just now" : "seen 18s ago",
      };
    }

    return {
      ...peer,
      state: refreshCounter % 2 === 0 ? "reconnecting" : "dormant",
      lastSeenLabel: refreshCounter % 2 === 0 ? "seen 55s ago" : "seen 3m ago",
    };
  });

  return cloneSnapshot();
}

export async function sendMessage(
  chatId: string,
  body: string,
  replyToMessageId: string | null = null,
): Promise<ClientSnapshot> {
  const chat = snapshot.chats.find((entry) => entry.id === chatId);
  const trimmed = body.trim();

  if (!chat || trimmed.length === 0) {
    return cloneSnapshot();
  }

  messageCounter += 1;
  const replyPreview = replyToMessageId
    ? (chat.messages.find((message) => message.id === replyToMessageId)?.body ??
      null)
    : null;
  const message = {
    id: `local-${messageCounter}`,
    author: "Rimus",
    body: trimmed,
    timestampLabel: "now",
    direction: "outbound" as const,
    deliveryState: "delivered" as const,
    forwardedFrom: null,
    replyPreview,
    reactions: [],
    attachments: [],
  };

  chat.messages.push(message);
  chat.summary = trimmed;
  chat.unreadCount = 0;
  chat.presenceLabel =
    chat.kind === "group" ? "3 of 4 peers reachable" : "secure session active";
  chat.presenceState = chat.kind === "group" ? "reconnecting" : "online";
  snapshot.notifications = {
    trayLabel: `${totalUnread()} unread`,
    unreadCount: totalUnread(),
    lastEvent: "Outgoing message delivered",
  };

  return cloneSnapshot();
}

export async function sendMedia(
  chatId: string,
  fileName: string,
  mimeType: string,
  bytesBase64: string,
  replyToMessageId: string | null = null,
): Promise<ClientSnapshot> {
  const chat = snapshot.chats.find((entry) => entry.id === chatId);
  if (
    !chat ||
    fileName.trim().length === 0 ||
    bytesBase64.trim().length === 0
  ) {
    return cloneSnapshot();
  }

  const byteLength = Math.floor((bytesBase64.length * 3) / 4);
  const direct = byteLength > 5 * 1024 * 1024;
  const replyPreview = replyToMessageId
    ? (chat.messages.find((message) => message.id === replyToMessageId)?.body ??
      null)
    : null;
  messageCounter += 1;
  chat.messages.push({
    id: `local-${messageCounter}`,
    author: "Rimus",
    body: attachmentBody(fileName, mimeType),
    timestampLabel: "now",
    direction: "outbound",
    deliveryState: "delivered",
    forwardedFrom: null,
    replyPreview,
    reactions: [],
    attachments: [
      {
        id: `att-${messageCounter}`,
        fileName,
        mimeType,
        sizeLabel:
          byteLength >= 1024 * 1024
            ? `${(byteLength / (1024 * 1024)).toFixed(1)} MB`
            : `${Math.max(1, Math.round(byteLength / 1024))} KB`,
        transferRoute: direct ? "p2p_quic_direct" : "server_blob_store",
        uploadProgress: 1.0,
        statusLabel: direct
          ? "direct QUIC handoff complete · mock-direct"
          : "encrypted relay blob ready · mock-blob",
        previewDataUrl:
          mimeType.startsWith("image/") ||
          mimeType.startsWith("audio/") ||
          mimeType === "application/pdf"
            ? `data:${mimeType};base64,${bytesBase64}`
            : null,
        blobId: direct ? null : "mock-blob",
      },
    ],
  });
  chat.summary = chat.messages[chat.messages.length - 1]?.body ?? chat.summary;
  chat.presenceLabel = direct
    ? "direct QUIC file lane active"
    : "relay blob storage active for small media";
  chat.presenceState = "online";
  snapshot.notifications = {
    trayLabel: `${totalUnread()} unread`,
    unreadCount: totalUnread(),
    lastEvent: `Shared media in ${chat.title}`,
  };

  return cloneSnapshot();
}

export async function verifyDevice(
  deviceId: string,
  action: VerificationAction,
): Promise<ClientSnapshot> {
  snapshot.verification.devices = snapshot.verification.devices.map(
    (device) => {
      if (device.deviceId !== deviceId) {
        return device;
      }

      return {
        ...device,
        state: "verified",
        method: action === "qr" ? "qr_code" : "safety_number",
      };
    },
  );

  snapshot.verification.trustedDeviceCount =
    snapshot.verification.devices.filter(
      (device) => device.state === "verified",
    ).length;
  snapshot.verification.pendingDeviceCount =
    snapshot.verification.devices.filter(
      (device) => device.state === "pending",
    ).length;

  snapshot.peers = snapshot.peers.map((peer) =>
    peer.deviceId === deviceId
      ? {
          ...peer,
          trustLabel: "verified",
        }
      : peer,
  );

  snapshot.chats = snapshot.chats.map((chat: ChatThreadView) =>
    chat.title === "Carol" && deviceId === "carol-workstation"
      ? {
          ...chat,
          securityLabel: "All known devices verified",
        }
      : chat,
  );

  return cloneSnapshot();
}

export async function previewInvite(
  inviteLink: string,
): Promise<ClientSnapshot> {
  snapshot.onboarding = {
    statusLabel: "Invite preview is ready.",
    invitePreview: {
      inviteId: "mock-invite",
      label: "Mock relay",
      serverAddr: "203.0.113.10:7443",
      serverName: "relay.local",
      expiresAtLabel: "Tomorrow",
      maxUses: 4,
    },
    contactInvitePreview: null,
  };
  if (inviteLink.trim().length === 0) {
    snapshot.onboarding.statusLabel = "Invite link is empty.";
    snapshot.onboarding.invitePreview = null;
  }
  snapshot.notifications = {
    trayLabel: `${totalUnread()} unread`,
    unreadCount: totalUnread(),
    lastEvent: "Invite preview is ready",
  };
  return cloneSnapshot();
}

export async function acceptInvite(
  inviteLink: string,
): Promise<ClientSnapshot> {
  await previewInvite(inviteLink);
  snapshot.transportStatus.serverStatus = "connected";
  snapshot.transportStatus.authStatus = "authenticated";
  snapshot.transportStatus.activeRoute = "server_relay";
  snapshot.serverStatus = "connected";
  snapshot.authStatus = "authenticated";
  snapshot.activeRoute = "server_relay";
  snapshot.onboarding.statusLabel =
    "Joined relay 203.0.113.10:7443 as rimus-laptop.";
  snapshot.notifications = {
    trayLabel: `${totalUnread()} unread`,
    unreadCount: totalUnread(),
    lastEvent: "Relay joined",
  };
  return cloneSnapshot();
}

export async function toggleReaction(
  chatId: string,
  messageId: string,
  reaction: string,
): Promise<ClientSnapshot> {
  const chat = snapshot.chats.find((entry) => entry.id === chatId);
  const trimmed = reaction.trim();
  if (!chat || trimmed.length === 0) {
    return cloneSnapshot();
  }

  const message = chat.messages.find((entry) => entry.id === messageId);
  if (!message) {
    return cloneSnapshot();
  }

  message.reactions = message.reactions.includes(trimmed)
    ? message.reactions.filter((entry) => entry !== trimmed)
    : [...message.reactions, trimmed];
  snapshot.notifications = {
    trayLabel: `${totalUnread()} unread`,
    unreadCount: totalUnread(),
    lastEvent: `Reaction updated in ${chat.title}`,
  };
  return cloneSnapshot();
}

export async function forwardMessage(
  sourceChatId: string,
  targetChatId: string,
  messageId: string,
): Promise<ClientSnapshot> {
  const sourceChat = snapshot.chats.find((entry) => entry.id === sourceChatId);
  const targetChat = snapshot.chats.find((entry) => entry.id === targetChatId);
  const sourceMessage = sourceChat?.messages.find(
    (entry) => entry.id === messageId,
  );
  if (!sourceChat || !targetChat || !sourceMessage) {
    return cloneSnapshot();
  }

  messageCounter += 1;
  targetChat.messages.push({
    ...JSON.parse(JSON.stringify(sourceMessage)),
    id: `local-${messageCounter}`,
    author: "Rimus",
    timestampLabel: "now",
    direction: "outbound",
    deliveryState: "delivered",
    forwardedFrom: sourceChat.title,
    reactions: [],
  });
  targetChat.summary = previewBody(sourceMessage.body);
  targetChat.unreadCount = 0;
  targetChat.presenceLabel = "secure session active";
  targetChat.presenceState = "online";
  snapshot.notifications = {
    trayLabel: `${totalUnread()} unread`,
    unreadCount: totalUnread(),
    lastEvent: `Forwarded message to ${targetChat.title}`,
  };
  return cloneSnapshot();
}

export async function checkForUpdates(): Promise<ClientSnapshot> {
  snapshot.updater = {
    ...snapshot.updater,
    lastCheckedLabel: "just now",
    statusLabel: snapshot.updater.feedUrl
      ? `Update feed configured at ${snapshot.updater.feedUrl}, but runtime auto-install is disabled in the mock shell.`
      : "No updater feed configured. Bundled artifacts are still available for signed release publishing.",
  };
  snapshot.notifications = {
    trayLabel: `${totalUnread()} unread`,
    unreadCount: totalUnread(),
    lastEvent: "Update status refreshed",
  };
  return cloneSnapshot();
}

function samplePhotoDataUrl(): string {
  return `data:image/svg+xml;base64,${btoa(
    "<svg xmlns='http://www.w3.org/2000/svg' width='480' height='320' viewBox='0 0 480 320'><defs><linearGradient id='g' x1='0%' y1='0%' x2='100%' y2='100%'><stop offset='0%' stop-color='#16354a'/><stop offset='100%' stop-color='#f07c3e'/></linearGradient></defs><rect width='480' height='320' rx='26' fill='url(#g)'/><circle cx='104' cy='92' r='34' fill='#ffe8a8' opacity='0.85'/><path d='M54 246 152 142l70 70 48-40 102 74H54Z' fill='#0e2231' opacity='0.92'/><path d='M162 246 248 156l54 54 34-24 88 60H162Z' fill='#e9f2f6' opacity='0.76'/><text x='34' y='42' fill='#f7f4ec' font-size='22' font-family='IBM Plex Sans, sans-serif'>Relay photo preview</text></svg>",
  )}`;
}

function sampleVoiceDataUrl(): string {
  const wavBytes = new Uint8Array([
    82, 73, 70, 70, 44, 0, 0, 0, 87, 65, 86, 69, 102, 109, 116, 32, 16, 0, 0, 0,
    1, 0, 1, 0, 64, 31, 0, 0, 128, 62, 0, 0, 2, 0, 16, 0, 100, 97, 116, 97, 8,
    0, 0, 0, 0, 0, 20, 10, 20, 10, 0, 0,
  ]);
  let binary = "";
  wavBytes.forEach((value) => {
    binary += String.fromCharCode(value);
  });
  return `data:audio/wav;base64,${btoa(binary)}`;
}

function samplePdfDataUrl(): string {
  const pdf = `%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 180] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>
endobj
4 0 obj
<< /Length 58 >>
stream
BT /F1 18 Tf 32 110 Td (Relay hardening brief preview) Tj ET
endstream
endobj
5 0 obj
<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>
endobj
xref
0 6
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000241 00000 n
0000000349 00000 n
trailer
<< /Size 6 /Root 1 0 R >>
startxref
419
%%EOF`;
  return `data:application/pdf;base64,${btoa(pdf)}`;
}

function attachmentBody(fileName: string, mimeType: string): string {
  if (mimeType.startsWith("image/")) {
    return `Shared photo: ${fileName}`;
  }
  if (mimeType === "application/pdf") {
    return `Shared document: ${fileName}`;
  }
  return `Shared file: ${fileName}`;
}

function previewBody(body: string): string {
  return body.length <= 72 ? body : `${body.slice(0, 69)}...`;
}

function totalUnread(): number {
  return snapshot.chats.reduce((sum, chat) => sum + chat.unreadCount, 0);
}

export async function startChatWithPeer(
  deviceId: string,
): Promise<ClientSnapshot> {
  const peer = snapshot.peers.find((p) => p.deviceId === deviceId);
  if (!peer) {
    throw new Error(`Peer ${deviceId} not found`);
  }

  const existingChat = snapshot.chats.find((c) =>
    c.participants.includes(peer.deviceName),
  );
  if (existingChat) {
    return cloneSnapshot();
  }

  messageCounter += 1;
  snapshot.chats.push({
    id: `chat-${deviceId}`,
    title: peer.deviceName,
    summary: `Secure session with ${peer.deviceName}`,
    presenceLabel: "secure session active",
    presenceState: "online",
    unreadCount: 0,
    securityLabel: "E2EE session established",
    kind: "direct",
    participants: ["Rimus", peer.deviceName],
    messages: [],
  });

  snapshot.notifications = {
    trayLabel: `${totalUnread()} unread`,
    unreadCount: totalUnread(),
    lastEvent: `Started chat with ${peer.deviceName}`,
  };

  return cloneSnapshot();
}
