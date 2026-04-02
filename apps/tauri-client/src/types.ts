export type LocaleCode = "en" | "ru";
export type ThemeMode = "midnight" | "daybreak";
export type ChatKind = "direct" | "group";
export type MessageDirection = "inbound" | "outbound" | "system";
export type PeerState = "live" | "reconnecting" | "dormant";
export type PresenceState = "online" | "reconnecting" | "offline";
export type VerificationState = "pending" | "verified";
export type VerificationMethod = "qr_code" | "safety_number";
export type VerificationAction = "qr" | "safety";

export interface TransportStatusView {
  discoveryMode: string;
  transportMode: string;
  cryptoMode: string;
  storageMode: string;
  serverStatus: string;
  authStatus: string;
  activeRoute: string;
}

export interface LocalProfileView {
  displayName: string;
  activeDeviceName: string;
  activeDeviceId: string;
  trustedDeviceCount: number;
  totalDeviceCount: number;
}

export interface InvitePreviewView {
  inviteId: string;
  label: string;
  serverAddr: string;
  serverName: string;
  expiresAtLabel: string;
  maxUses: number;
}

export interface ContactInvitePreviewView {
  memberId: string;
  deviceId: string;
  displayName: string;
  serverAddr: string;
  serverName: string;
  expiresAtLabel: string;
}

export interface OnboardingView {
  statusLabel: string;
  invitePreview: InvitePreviewView | null;
  contactInvitePreview: ContactInvitePreviewView | null;
}

export interface UpdaterView {
  currentVersion: string;
  channel: string;
  statusLabel: string;
  lastCheckedLabel: string;
  canAutoUpdate: boolean;
  feedUrl: string | null;
}

export interface NotificationCenterView {
  trayLabel: string;
  unreadCount: number;
  lastEvent: string;
}

export interface MessageAttachmentView {
  id: string;
  fileName: string;
  mimeType: string;
  sizeLabel: string;
  transferRoute: string;
  statusLabel: string;
  previewDataUrl: string | null;
  blobId: string | null;
  uploadProgress: number;
}

export interface MessageView {
  id: string;
  author: string;
  body: string;
  timestampLabel: string;
  direction: MessageDirection;
  deliveryState: "queued" | "sent" | "delivered" | "seen";
  forwardedFrom: string | null;
  replyPreview: string | null;
  reactions: string[];
  attachments: MessageAttachmentView[];
}

export interface ChatThreadView {
  id: string;
  title: string;
  summary: string;
  presenceLabel: string;
  presenceState: PresenceState;
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
  serverStatus: string;
  authStatus: string;
  activeRoute: string;
  notifications: NotificationCenterView;
  localProfile: LocalProfileView;
  chats: ChatThreadView[];
  peers: PeerView[];
  verification: VerificationWorkspaceView;
  onboarding: OnboardingView;
  updater: UpdaterView;
}
