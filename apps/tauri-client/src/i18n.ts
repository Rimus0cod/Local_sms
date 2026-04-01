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
  recordVoice: string;
  stopVoiceRecording: string;
  attachMedia: string;
  send: string;
  refreshPeers: string;
  noMessages: string;
  participants: string;
  replyAction: string;
  forwardAction: string;
  forwardedFrom: string;
  replyingTo: string;
  forwardingMessage: string;
  cancelAction: string;
  reactionsLabel: string;
  updaterTitle: string;
  updaterCheck: string;
  updaterChannel: string;
  updaterVersion: string;
  trustedDevices: string;
  pendingDevices: string;
  scanQr: string;
  safetyCheck: string;
  qrPayload: string;
  safetyNumber: string;
  verifiedVia: string;
  noChatSelected: string;
  noVerificationDevice: string;
  searchPlaceholder: string;
  youLabel: string;
  settingsTitle: string;
  settingsGeneral: string;
  settingsDevices: string;
  settingsNetwork: string;
  settingsRelay: string;
  settingsUpdates: string;
  settingsAppearance: string;
  settingsTheme: string;
  settingsLanguage: string;
  themeDark: string;
  themeLight: string;
  settingsVerifiedLabel: string;
  settingsPendingLabel: string;
  settingsOnlineLabel: string;
  settingsJoinRelay: string;
  settingsPreviewInvite: string;
  settingsJoinButton: string;
  settingsLastCheck: string;
  inviteFriend: string;
  inviteTitle: string;
  inviteSubtitle: string;
  inviteLinkLabel: string;
  inviteCopyLink: string;
  inviteCopied: string;
  inviteShareTelegram: string;
  inviteShareWhatsApp: string;
  inviteShareVK: string;
  inviteQrTitle: string;
  inviteScanHint: string;
  inviteEmptyTitle: string;
  inviteEmptySubtitle: string;
  inviteSent: string;
  inviteClose: string;
  uploadingLabel: string;
  uploadCompleteLabel: string;
  uploadingProgress: string;
  mediaViaRelay: string;
  mediaViaDirect: string;
  scrollToBottom: string;
  groupLabel: string;
  noRelayForGroup: string;
  peerStates: Record<PeerState, string>;
  verificationStates: Record<VerificationState, string>;
  verificationMethods: Record<VerificationMethod, string>;
  messageDirections: Record<MessageDirection, string>;
  deliveryStates: Record<"queued" | "sent" | "delivered" | "seen", string>;
};

export const COPY: Record<LocaleCode, CopyBundle> = {
  en: {
    title: "Local Messenger",
    subtitle: "Local-first secure group comms for trusted devices",
    lanBadge: "LAN-first",
    e2eeBadge: "End-to-End Encrypted",
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
    composerPlaceholder: "Write a message...",
    recordVoice: "Record voice",
    stopVoiceRecording: "Stop recording",
    attachMedia: "Attach media",
    send: "Send",
    refreshPeers: "Refresh LAN scan",
    noMessages: "No messages yet.",
    participants: "Participants",
    replyAction: "Reply",
    forwardAction: "Forward",
    forwardedFrom: "Forwarded from",
    replyingTo: "Replying to",
    forwardingMessage: "Forwarding",
    cancelAction: "Cancel",
    reactionsLabel: "Reactions",
    updaterTitle: "App updates",
    updaterCheck: "Check updates",
    updaterChannel: "Channel",
    updaterVersion: "Version",
    trustedDevices: "Trusted devices",
    pendingDevices: "Pending review",
    scanQr: "Verify via QR",
    safetyCheck: "Verify via safety number",
    qrPayload: "QR payload",
    safetyNumber: "Safety number",
    verifiedVia: "Verified via",
    noChatSelected: "Select a chat to start messaging",
    noVerificationDevice:
      "Select a device to inspect its verification material.",
    searchPlaceholder: "Search",
    youLabel: "You",
    settingsTitle: "Settings",
    settingsGeneral: "General",
    settingsDevices: "Privacy and Security",
    settingsNetwork: "Network and Peers",
    settingsRelay: "Relay Server",
    settingsUpdates: "App Updates",
    settingsAppearance: "Appearance",
    settingsTheme: "Theme",
    settingsLanguage: "Language",
    themeDark: "Dark",
    themeLight: "Light",
    settingsVerifiedLabel: "verified",
    settingsPendingLabel: "pending",
    settingsOnlineLabel: "online",
    settingsJoinRelay: "Join a relay server",
    settingsPreviewInvite: "Preview",
    settingsJoinButton: "Join relay",
    settingsLastCheck: "Last check",
    inviteFriend: "Invite",
    inviteTitle: "Invite a friend",
    inviteSubtitle: "Share a link to let someone join your trusted group",
    inviteLinkLabel: "Invite link",
    inviteCopyLink: "Copy link",
    inviteCopied: "Copied!",
    inviteShareTelegram: "Telegram",
    inviteShareWhatsApp: "WhatsApp",
    inviteShareVK: "VK",
    inviteQrTitle: "QR Code",
    inviteScanHint: "Scan this code to join",
    inviteEmptyTitle: "No friends yet",
    inviteEmptySubtitle: "Invite someone to start chatting",
    inviteSent: "Link copied to clipboard",
    inviteClose: "Close",
    uploadingLabel: "Uploading…",
    uploadCompleteLabel: "Uploaded",
    uploadingProgress: "Uploading {n}%",
    mediaViaRelay: "via relay",
    mediaViaDirect: "P2P direct",
    scrollToBottom: "Jump to latest",
    groupLabel: "Group",
    noRelayForGroup: "Group media requires a relay server",
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
      seen: "Read",
    },
  },
  ru: {
    title: "Local Messenger",
    subtitle: "Локальный защищенный чат для доверенных устройств",
    lanBadge: "Локальная сеть",
    e2eeBadge: "Шифрование E2E",
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
    composerPlaceholder: "Напишите сообщение...",
    recordVoice: "Записать голос",
    stopVoiceRecording: "Остановить запись",
    attachMedia: "Прикрепить медиа",
    send: "Отправить",
    refreshPeers: "Обновить поиск",
    noMessages: "Сообщений пока нет.",
    participants: "Участники",
    replyAction: "Ответить",
    forwardAction: "Переслать",
    forwardedFrom: "Переслано из",
    replyingTo: "Ответ",
    forwardingMessage: "Пересылка",
    cancelAction: "Отмена",
    reactionsLabel: "Реакции",
    updaterTitle: "Обновления",
    updaterCheck: "Проверить обновления",
    updaterChannel: "Канал",
    updaterVersion: "Версия",
    trustedDevices: "Доверенные устройства",
    pendingDevices: "Ожидают проверки",
    scanQr: "Подтвердить по QR",
    safetyCheck: "Подтвердить по номеру безопасности",
    qrPayload: "QR-данные",
    safetyNumber: "Номер безопасности",
    verifiedVia: "Подтверждено через",
    noChatSelected: "Выберите чат для начала общения",
    noVerificationDevice:
      "Выберите устройство, чтобы увидеть данные для верификации.",
    searchPlaceholder: "Поиск",
    youLabel: "Вы",
    settingsTitle: "Настройки",
    settingsGeneral: "Основные",
    settingsDevices: "Конфиденциальность",
    settingsNetwork: "Сеть и пиры",
    settingsRelay: "Релей-сервер",
    settingsUpdates: "Обновления",
    settingsAppearance: "Внешний вид",
    settingsTheme: "Тема",
    settingsLanguage: "Язык",
    themeDark: "Тёмная",
    themeLight: "Светлая",
    settingsVerifiedLabel: "проверено",
    settingsPendingLabel: "ожидают",
    settingsOnlineLabel: "в сети",
    settingsJoinRelay: "Подключение к релей-серверу",
    settingsPreviewInvite: "Просмотр",
    settingsJoinButton: "Подключиться",
    settingsLastCheck: "Последняя проверка",
    inviteFriend: "Пригласить",
    inviteTitle: "Пригласить друга",
    inviteSubtitle:
      "Поделитесь ссылкой, чтобы кто-то присоединился к вашей группе",
    inviteLinkLabel: "Ссылка-приглашение",
    inviteCopyLink: "Копировать ссылку",
    inviteCopied: "Скопировано!",
    inviteShareTelegram: "Telegram",
    inviteShareWhatsApp: "WhatsApp",
    inviteShareVK: "ВКонтакте",
    inviteQrTitle: "QR-код",
    inviteScanHint: "Отсканируйте код для входа",
    inviteEmptyTitle: "Пока нет друзей",
    inviteEmptySubtitle: "Пригласите кого-нибудь, чтобы начать общение",
    inviteSent: "Ссылка скопирована",
    inviteClose: "Закрыть",
    uploadingLabel: "Загрузка…",
    uploadCompleteLabel: "Загружено",
    uploadingProgress: "Загрузка {n}%",
    mediaViaRelay: "через сервер",
    mediaViaDirect: "P2P напрямую",
    scrollToBottom: "К последнему",
    groupLabel: "Группа",
    noRelayForGroup: "Медиа-файлы группы требуют сервер-ретранслятор",
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
      seen: "Прочитано",
    },
  },
};
