import { create } from "zustand";

import {
  loadClientSnapshot,
  refreshPeerDiscovery,
  sendClientMessage,
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
  verifyDevice: (action: VerificationAction) => Promise<void>;
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
    set({ selectedChatId: chatId });
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
  toggleTheme: () => {
    set((state) => ({
      theme: state.theme === "midnight" ? "daybreak" : "midnight",
    }));
  },
  setLocale: (locale) => {
    set({ locale });
  },
}));
