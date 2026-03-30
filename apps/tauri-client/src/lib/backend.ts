import { invoke } from "@tauri-apps/api/core";

import type { ClientSnapshot, VerificationAction } from "../types";
import * as mockBackend from "./mockBackend";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

function hasTauriRuntime(): boolean {
  return typeof window !== "undefined" && Boolean(window.__TAURI_INTERNALS__);
}

export async function loadClientSnapshot(): Promise<ClientSnapshot> {
  if (!hasTauriRuntime()) {
    return mockBackend.loadSnapshot();
  }

  return invoke<ClientSnapshot>("load_client_snapshot");
}

export async function refreshPeerDiscovery(): Promise<ClientSnapshot> {
  if (!hasTauriRuntime()) {
    return mockBackend.refreshPeers();
  }

  return invoke<ClientSnapshot>("refresh_peer_discovery");
}

export async function sendClientMessage(
  chatId: string,
  body: string,
): Promise<ClientSnapshot> {
  if (!hasTauriRuntime()) {
    return mockBackend.sendMessage(chatId, body);
  }

  return invoke<ClientSnapshot>("send_message", { chat_id: chatId, body });
}

export async function verifyClientDevice(
  deviceId: string,
  action: VerificationAction,
): Promise<ClientSnapshot> {
  if (!hasTauriRuntime()) {
    return mockBackend.verifyDevice(deviceId, action);
  }

  return invoke<ClientSnapshot>("verify_device", {
    device_id: deviceId,
    method: action,
  });
}
