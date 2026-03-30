export type LocaleCode = "en" | "ru";
export type ThemeMode = "midnight" | "daybreak";
export type ChatKind = "direct" | "group";
export type MessageDirection = "inbound" | "outbound" | "system";
export type PeerState = "live" | "reconnecting" | "dormant";
export type VerificationState = "pending" | "verified";
export type VerificationMethod = "qr_code" | "safety_number";
export type VerificationAction = "qr" | "safety";

export interface TransportStatusView {
  discoveryMode: string;
  transportMode: string;
  cryptoMode: string;
  storageMode: string;
}

export interface LocalProfileView {
  displayName: string;
  activeDeviceName: string;
  activeDeviceId: string;
  trustedDeviceCount: number;
  totalDeviceCount: number;
}

export interface MessageView {
  id: string;
  author: string;
  body: string;
  timestampLabel: string;
  direction: MessageDirection;
  deliveryState: "queued" | "sent" | "delivered";
  replyPreview: string | null;
  reactions: string[];
}

export interface ChatThreadView {
  id: string;
  title: string;
  summary: string;
  presenceLabel: string;
  unreadCount: number;
  securityLabel: string;
  kind: ChatKind;
  participants: string[];
  messages: MessageView[];
}

export interface PeerView {
  memberId: string;
  deviceId: string;
  deviceName: string;
  endpoint: string;
  hostname: string | null;
  capabilities: string[];
  state: PeerState;
  trustLabel: string;
  lastSeenLabel: string;
}

export interface VerificationDeviceView {
  memberId: string;
  memberName: string;
  deviceId: string;
  deviceName: string;
  state: VerificationState;
  method: VerificationMethod | null;
  safetyNumber: string;
  qrPayloadHex: string;
}

export interface VerificationWorkspaceView {
  trustedDeviceCount: number;
  pendingDeviceCount: number;
  devices: VerificationDeviceView[];
}

export interface ClientSnapshot {
  transportStatus: TransportStatusView;
  localProfile: LocalProfileView;
  chats: ChatThreadView[];
  peers: PeerView[];
  verification: VerificationWorkspaceView;
}
