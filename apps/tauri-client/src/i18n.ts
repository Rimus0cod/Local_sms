import type {
  LocaleCode,
  MessageDirection,
  PeerState,
  VerificationMethod,
  VerificationState,
} from "./types";

export type CopyBundle = {
  title: string;
  subtitle: string;
  lanBadge: string;
  e2eeBadge: string;
  themeLabel: string;
  localeLabel: string;
  operatorLabel: string;
  trustedLocalDevicesLabel: string;
  discoveryScopeLabel: string;
  chats: string;
  peers: string;
  verification: string;
  secureRooms: string;
  discoveryTransport: string;
  trustWorkspace: string;
  discoveryLabel: string;
  transportLabel: string;
  cryptoLabel: string;
  storageLabel: string;
  composerPlaceholder: string;
  send: string;
  refreshPeers: string;
  noMessages: string;
  participants: string;
  trustedDevices: string;
  pendingDevices: string;
  scanQr: string;
  safetyCheck: string;
  qrPayload: string;
  safetyNumber: string;
  verifiedVia: string;
  noChatSelected: string;
  noVerificationDevice: string;
  peerStates: Record<PeerState, string>;
  verificationStates: Record<VerificationState, string>;
  verificationMethods: Record<VerificationMethod, string>;
  messageDirections: Record<MessageDirection, string>;
  deliveryStates: Record<"queued" | "sent" | "delivered", string>;
};

export const COPY: Record<LocaleCode, CopyBundle> = {
  en: {
    title: "Local Messenger",
    subtitle: "Local-first secure group comms for trusted devices",
    lanBadge: "LAN-first",
    e2eeBadge: "E2EE active",
    themeLabel: "Theme",
    localeLabel: "Locale",
    operatorLabel: "Operator",
    trustedLocalDevicesLabel: "Trusted local devices",
    discoveryScopeLabel: "Discovery scope",
    chats: "Chats",
    peers: "Peers in LAN",
    verification: "Device verification",
    secureRooms: "Secure rooms",
    discoveryTransport: "Discovery and transport",
    trustWorkspace: "Trust workspace",
    discoveryLabel: "Discovery",
    transportLabel: "Transport",
    cryptoLabel: "Crypto",
    storageLabel: "Storage",
    composerPlaceholder: "Write a message for the secure room...",
    send: "Send",
    refreshPeers: "Refresh LAN scan",
    noMessages: "No messages yet.",
    participants: "Participants",
    trustedDevices: "Trusted devices",
    pendingDevices: "Pending review",
    scanQr: "Mark via QR",
    safetyCheck: "Mark via safety number",
    qrPayload: "QR payload",
    safetyNumber: "Safety number",
    verifiedVia: "Verified via",
    noChatSelected: "Select a chat to inspect message flow and peer posture.",
    noVerificationDevice: "Select a device to inspect its verification material.",
    peerStates: {
      live: "Reachable",
      reconnecting: "Reconnecting",
      dormant: "Dormant",
    },
    verificationStates: {
      pending: "Pending",
      verified: "Verified",
    },
    verificationMethods: {
      qr_code: "QR code",
      safety_number: "Safety number",
    },
    messageDirections: {
      inbound: "Inbound",
      outbound: "Outbound",
      system: "System",
    },
    deliveryStates: {
      queued: "Queued",
      sent: "Sent",
      delivered: "Delivered",
    },
  },
  ru: {
    title: "Local Messenger",
    subtitle: "Локальный защищенный чат для доверенных устройств",
    lanBadge: "Локальная сеть",
    e2eeBadge: "E2EE активно",
    themeLabel: "Тема",
    localeLabel: "Язык",
    operatorLabel: "Оператор",
    trustedLocalDevicesLabel: "Доверенные локальные устройства",
    discoveryScopeLabel: "Контур обнаружения",
    chats: "Чаты",
    peers: "Пиры в LAN",
    verification: "Проверка устройств",
    secureRooms: "Защищенные комнаты",
    discoveryTransport: "Обнаружение и транспорт",
    trustWorkspace: "Пространство доверия",
    discoveryLabel: "Обнаружение",
    transportLabel: "Транспорт",
    cryptoLabel: "Криптография",
    storageLabel: "Хранилище",
    composerPlaceholder: "Напишите сообщение в защищенный чат...",
    send: "Отправить",
    refreshPeers: "Обновить поиск",
    noMessages: "Сообщений пока нет.",
    participants: "Участники",
    trustedDevices: "Доверенные устройства",
    pendingDevices: "Ожидают проверки",
    scanQr: "Подтвердить по QR",
    safetyCheck: "Подтвердить по safety number",
    qrPayload: "QR-данные",
    safetyNumber: "Номер безопасности",
    verifiedVia: "Подтверждено через",
    noChatSelected:
      "Выберите чат, чтобы посмотреть поток сообщений и статус соединения.",
    noVerificationDevice:
      "Выберите устройство, чтобы увидеть данные для верификации.",
    peerStates: {
      live: "Доступен",
      reconnecting: "Переподключение",
      dormant: "Неактивен",
    },
    verificationStates: {
      pending: "Ожидает",
      verified: "Проверено",
    },
    verificationMethods: {
      qr_code: "QR-код",
      safety_number: "Номер безопасности",
    },
    messageDirections: {
      inbound: "Входящее",
      outbound: "Исходящее",
      system: "Системное",
    },
    deliveryStates: {
      queued: "В очереди",
      sent: "Отправлено",
      delivered: "Доставлено",
    },
  },
};
