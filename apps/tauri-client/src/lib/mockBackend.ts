import type {
  ChatThreadView,
  ClientSnapshot,
  VerificationAction,
} from "../types";

function createInitialSnapshot(): ClientSnapshot {
  return {
    transportStatus: {
      discoveryMode: "mDNS peer discovery",
      transportMode: "QUIC transport",
      cryptoMode: "X3DH bootstrap + Double Ratchet",
      storageMode: "Encrypted SQLite at rest",
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
            replyPreview: null,
            reactions: ["ack"],
          },
          {
            id: "m-2",
            author: "Rimus",
            body: "Good. I want to push the sender-key rotation UI next.",
            timestampLabel: "09:22",
            direction: "outbound",
            deliveryState: "delivered",
            replyPreview: "I am back on the office Wi-Fi. QUIC path is stable now.",
            reactions: [],
          },
          {
            id: "m-3",
            author: "Bob",
            body: "LAN sync window looks clean. Ready for file relay tests.",
            timestampLabel: "09:25",
            direction: "inbound",
            deliveryState: "delivered",
            replyPreview: null,
            reactions: ["+1"],
          },
        ],
      },
      {
        id: "chat-lan-crew",
        title: "LAN Crew",
        summary: "Epoch 4 is active after Carol workstation rejoin.",
        presenceLabel: "3 of 4 peers reachable",
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
            replyPreview: null,
            reactions: [],
          },
          {
            id: "g-2",
            author: "Carol",
            body: "My workstation is reachable again. Discovery TTL looks normal.",
            timestampLabel: "08:45",
            direction: "inbound",
            deliveryState: "delivered",
            replyPreview: null,
            reactions: [],
          },
        ],
      },
      {
        id: "chat-carol",
        title: "Carol",
        summary: "Need to verify the tablet before enabling attachments.",
        presenceLabel: "reconnecting",
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
            replyPreview: null,
            reactions: [],
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
        lastSeenLabel: refreshCounter % 2 === 0 ? "seen just now" : "seen 18s ago",
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
): Promise<ClientSnapshot> {
  const chat = snapshot.chats.find((entry) => entry.id === chatId);
  const trimmed = body.trim();

  if (!chat || trimmed.length === 0) {
    return cloneSnapshot();
  }

  messageCounter += 1;
  const message = {
    id: `local-${messageCounter}`,
    author: "Rimus",
    body: trimmed,
    timestampLabel: "now",
    direction: "outbound" as const,
    deliveryState: "delivered" as const,
    replyPreview: null,
    reactions: [],
  };

  chat.messages.push(message);
  chat.summary = trimmed;
  chat.unreadCount = 0;
  chat.presenceLabel =
    chat.kind === "group" ? "3 of 4 peers reachable" : "secure session active";

  return cloneSnapshot();
}

export async function verifyDevice(
  deviceId: string,
  action: VerificationAction,
): Promise<ClientSnapshot> {
  snapshot.verification.devices = snapshot.verification.devices.map((device) => {
    if (device.deviceId !== deviceId) {
      return device;
    }

    return {
      ...device,
      state: "verified",
      method: action === "qr" ? "qr_code" : "safety_number",
    };
  });

  snapshot.verification.trustedDeviceCount = snapshot.verification.devices.filter(
    (device) => device.state === "verified",
  ).length;
  snapshot.verification.pendingDeviceCount = snapshot.verification.devices.filter(
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
