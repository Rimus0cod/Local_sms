import { create } from "zustand";

import {
  acceptInviteLink,
  checkForClientUpdates,
  forwardClientMessage,
  loadClientSnapshot,
  previewInviteLink,
  refreshPeerDiscovery,
  sendClientMedia,
  sendClientMessage,
  startChatWithPeer,
  toggleClientReaction,
  verifyClientDevice,
} from "../lib/backend";
import type {
  ClientSnapshot,
  LocaleCode,
  ThemeMode,
  VerificationAction,
} from "../types";

type ClientStore = {
  snapshot: ClientSnapshot | null;
  selectedChatId: string | null;
  selectedVerificationDeviceId: string | null;
  locale: LocaleCode;
  theme: ThemeMode;
  busy: boolean;
  error: string | null;
  load: () => Promise<void>;
  selectChat: (chatId: string) => void;
  selectVerificationDevice: (deviceId: string) => void;
  refreshPeers: () => Promise<void>;
  sendMessage: (body: string) => Promise<void>;
  sendMedia: (
    fileName: string,
    mimeType: string,
    bytesBase64: string,
    replyToMessageId?: string | null,
  ) => Promise<void>;
  sendReply: (messageId: string, body: string) => Promise<void>;
  toggleReaction: (messageId: string, reaction: string) => Promise<void>;
  forwardMessage: (sourceChatId: string, messageId: string) => Promise<void>;
  verifyDevice: (action: VerificationAction) => Promise<void>;
  previewInvite: (inviteLink: string) => Promise<void>;
  acceptInvite: (inviteLink: string) => Promise<void>;
  checkForUpdates: () => Promise<void>;
  startChatWithPeer: (deviceId: string) => Promise<void>;
  toggleTheme: () => void;
  setLocale: (locale: LocaleCode) => void;
};

function synchronizeSelection(
  snapshot: ClientSnapshot,
  selectedChatId: string | null,
  selectedVerificationDeviceId: string | null,
) {
  const nextChatId = snapshot.chats.some((chat) => chat.id === selectedChatId)
    ? selectedChatId
    : snapshot.chats[0]?.id ?? null;
  const nextVerificationDeviceId = snapshot.verification.devices.some(
    (device) => device.deviceId === selectedVerificationDeviceId,
  )
    ? selectedVerificationDeviceId
    : snapshot.verification.devices[0]?.deviceId ?? null;

  return {
    nextChatId,
    nextVerificationDeviceId,
  };
}

export const useClientStore = create<ClientStore>((set, get) => ({
  snapshot: null,
  selectedChatId: null,
  selectedVerificationDeviceId: null,
  locale: "ru",
  theme: "midnight",
  busy: false,
  error: null,
  load: async () => {
    set({ busy: true, error: null });

    try {
      const snapshot = await loadClientSnapshot();
      const { nextChatId, nextVerificationDeviceId } = synchronizeSelection(
        snapshot,
        get().selectedChatId,
        get().selectedVerificationDeviceId,
      );

      set({
        snapshot,
        selectedChatId: nextChatId,
        selectedVerificationDeviceId: nextVerificationDeviceId,
        busy: false,
      });
    } catch (error) {
      set({
        busy: false,
        error: error instanceof Error ? error.message : "Unknown client error",
      });
    }
  },
  selectChat: (chatId) => {
    set((state) => {
      if (!state.snapshot) {
        return { selectedChatId: chatId };
      }

      const chats = state.snapshot.chats.map((chat) =>
        chat.id === chatId ? { ...chat, unreadCount: 0 } : chat,
      );
      const unreadCount = chats.reduce((sum, chat) => sum + chat.unreadCount, 0);

      return {
        selectedChatId: chatId,
        snapshot: {
          ...state.snapshot,
          chats,
          notifications: {
            ...state.snapshot.notifications,
            unreadCount,
            trayLabel: unreadCount === 0 ? "Tray idle" : `${unreadCount} unread`,
          },
        },
      };
    });
  },
  selectVerificationDevice: (deviceId) => {
    set({ selectedVerificationDeviceId: deviceId });
  },
  refreshPeers: async () => {
    set({ busy: true, error: null });

    try {
      const snapshot = await refreshPeerDiscovery();
      const { nextChatId, nextVerificationDeviceId } = synchronizeSelection(
        snapshot,
        get().selectedChatId,
        get().selectedVerificationDeviceId,
      );

      set({
        snapshot,
        selectedChatId: nextChatId,
        selectedVerificationDeviceId: nextVerificationDeviceId,
        busy: false,
      });
    } catch (error) {
      set({
        busy: false,
        error: error instanceof Error ? error.message : "Peer refresh failed",
      });
    }
  },
  sendMessage: async (body) => {
    const state = get();
    if (!state.selectedChatId) {
      return;
    }

    set({ busy: true, error: null });

    try {
      const snapshot = await sendClientMessage(state.selectedChatId, body);
      const { nextChatId, nextVerificationDeviceId } = synchronizeSelection(
        snapshot,
        state.selectedChatId,
        state.selectedVerificationDeviceId,
      );

      set({
        snapshot,
        selectedChatId: nextChatId,
        selectedVerificationDeviceId: nextVerificationDeviceId,
        busy: false,
      });
    } catch (error) {
      set({
        busy: false,
        error: error instanceof Error ? error.message : "Message send failed",
      });
    }
  },
  sendMedia: async (fileName, mimeType, bytesBase64, replyToMessageId = null) => {
    const state = get();
    if (!state.selectedChatId) {
      return;
    }

    set({ busy: true, error: null });

    try {
      const snapshot = await sendClientMedia(
        state.selectedChatId,
        fileName,
        mimeType,
        bytesBase64,
        replyToMessageId,
      );
      const { nextChatId, nextVerificationDeviceId } = synchronizeSelection(
        snapshot,
        state.selectedChatId,
        state.selectedVerificationDeviceId,
      );

      set({
        snapshot,
        selectedChatId: nextChatId,
        selectedVerificationDeviceId: nextVerificationDeviceId,
        busy: false,
      });
    } catch (error) {
      set({
        busy: false,
        error: error instanceof Error ? error.message : "Media send failed",
      });
    }
  },
  sendReply: async (messageId, body) => {
    const state = get();
    if (!state.selectedChatId) {
      return;
    }

    set({ busy: true, error: null });

    try {
      const snapshot = await sendClientMessage(
        state.selectedChatId,
        body,
        messageId,
      );
      const { nextChatId, nextVerificationDeviceId } = synchronizeSelection(
        snapshot,
        state.selectedChatId,
        state.selectedVerificationDeviceId,
      );

      set({
        snapshot,
        selectedChatId: nextChatId,
        selectedVerificationDeviceId: nextVerificationDeviceId,
        busy: false,
      });
    } catch (error) {
      set({
        busy: false,
        error: error instanceof Error ? error.message : "Reply send failed",
      });
    }
  },
  toggleReaction: async (messageId, reaction) => {
    const state = get();
    if (!state.selectedChatId) {
      return;
    }

    set({ busy: true, error: null });

    try {
      const snapshot = await toggleClientReaction(
        state.selectedChatId,
        messageId,
        reaction,
      );
      const { nextChatId, nextVerificationDeviceId } = synchronizeSelection(
        snapshot,
        state.selectedChatId,
        state.selectedVerificationDeviceId,
      );

      set({
        snapshot,
        selectedChatId: nextChatId,
        selectedVerificationDeviceId: nextVerificationDeviceId,
        busy: false,
      });
    } catch (error) {
      set({
        busy: false,
        error: error instanceof Error ? error.message : "Reaction update failed",
      });
    }
  },
  forwardMessage: async (sourceChatId, messageId) => {
    const state = get();
    if (!state.selectedChatId) {
      return;
    }

    set({ busy: true, error: null });

    try {
      const snapshot = await forwardClientMessage(
        sourceChatId,
        state.selectedChatId,
        messageId,
      );
      const { nextChatId, nextVerificationDeviceId } = synchronizeSelection(
        snapshot,
        state.selectedChatId,
        state.selectedVerificationDeviceId,
      );

      set({
        snapshot,
        selectedChatId: nextChatId,
        selectedVerificationDeviceId: nextVerificationDeviceId,
        busy: false,
      });
    } catch (error) {
      set({
        busy: false,
        error: error instanceof Error ? error.message : "Forward failed",
      });
    }
  },
  verifyDevice: async (action) => {
    const state = get();
    if (!state.selectedVerificationDeviceId) {
      return;
    }

    set({ busy: true, error: null });

    try {
      const snapshot = await verifyClientDevice(
        state.selectedVerificationDeviceId,
        action,
      );
      const { nextChatId, nextVerificationDeviceId } = synchronizeSelection(
        snapshot,
        state.selectedChatId,
        state.selectedVerificationDeviceId,
      );

      set({
        snapshot,
        selectedChatId: nextChatId,
        selectedVerificationDeviceId: nextVerificationDeviceId,
        busy: false,
      });
    } catch (error) {
      set({
        busy: false,
        error: error instanceof Error ? error.message : "Verification failed",
      });
    }
  },
  previewInvite: async (inviteLink) => {
    set({ busy: true, error: null });

    try {
      const snapshot = await previewInviteLink(inviteLink);
      const { nextChatId, nextVerificationDeviceId } = synchronizeSelection(
        snapshot,
        get().selectedChatId,
        get().selectedVerificationDeviceId,
      );

      set({
        snapshot,
        selectedChatId: nextChatId,
        selectedVerificationDeviceId: nextVerificationDeviceId,
        busy: false,
      });
    } catch (error) {
      set({
        busy: false,
        error: error instanceof Error ? error.message : "Invite preview failed",
      });
    }
  },
  acceptInvite: async (inviteLink) => {
    set({ busy: true, error: null });

    try {
      const snapshot = await acceptInviteLink(inviteLink);
      const { nextChatId, nextVerificationDeviceId } = synchronizeSelection(
        snapshot,
        get().selectedChatId,
        get().selectedVerificationDeviceId,
      );

      set({
        snapshot,
        selectedChatId: nextChatId,
        selectedVerificationDeviceId: nextVerificationDeviceId,
        busy: false,
      });
    } catch (error) {
      set({
        busy: false,
        error: error instanceof Error ? error.message : "Invite acceptance failed",
      });
    }
  },
  checkForUpdates: async () => {
    set({ busy: true, error: null });

    try {
      const snapshot = await checkForClientUpdates();
      const { nextChatId, nextVerificationDeviceId } = synchronizeSelection(
        snapshot,
        get().selectedChatId,
        get().selectedVerificationDeviceId,
      );

      set({
        snapshot,
        selectedChatId: nextChatId,
        selectedVerificationDeviceId: nextVerificationDeviceId,
        busy: false,
      });
    } catch (error) {
      set({
        busy: false,
        error: error instanceof Error ? error.message : "Update check failed",
      });
    }
  },
  startChatWithPeer: async (deviceId) => {
    set({ busy: true, error: null });

    try {
      const snapshot = await startChatWithPeer(deviceId);
      const { nextChatId, nextVerificationDeviceId } = synchronizeSelection(
        snapshot,
        get().selectedChatId,
        get().selectedVerificationDeviceId,
      );

      set({
        snapshot,
        selectedChatId: nextChatId,
        selectedVerificationDeviceId: nextVerificationDeviceId,
        busy: false,
      });
    } catch (error) {
      set({
        busy: false,
        error: error instanceof Error ? error.message : "Failed to start chat",
      });
    }
  },
  toggleTheme: () => {
    set((state) => ({
      theme: state.theme === "midnight" ? "daybreak" : "midnight",
    }));
  },
  setLocale: (locale) => {
    set({ locale });
  },
}));
