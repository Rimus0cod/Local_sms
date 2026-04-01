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
  replyToMessageId?: string | null,
): Promise<ClientSnapshot> {
  if (!hasTauriRuntime()) {
    return mockBackend.sendMessage(chatId, body, replyToMessageId ?? null);
  }

  return invoke<ClientSnapshot>("send_message", {
    chat_id: chatId,
    body,
    reply_to_message_id: replyToMessageId ?? null,
  });
}

export async function sendClientMedia(
  chatId: string,
  fileName: string,
  mimeType: string,
  bytesBase64: string,
  replyToMessageId?: string | null,
): Promise<ClientSnapshot> {
  if (!hasTauriRuntime()) {
    return mockBackend.sendMedia(
      chatId,
      fileName,
      mimeType,
      bytesBase64,
      replyToMessageId ?? null,
    );
  }

  return invoke<ClientSnapshot>("send_media", {
    chat_id: chatId,
    file_name: fileName,
    mime_type: mimeType,
    bytes_base64: bytesBase64,
    reply_to_message_id: replyToMessageId ?? null,
  });
}

export async function toggleClientReaction(
  chatId: string,
  messageId: string,
  reaction: string,
): Promise<ClientSnapshot> {
  if (!hasTauriRuntime()) {
    return mockBackend.toggleReaction(chatId, messageId, reaction);
  }

  return invoke<ClientSnapshot>("toggle_reaction", {
    chat_id: chatId,
    message_id: messageId,
    reaction,
  });
}

export async function forwardClientMessage(
  sourceChatId: string,
  targetChatId: string,
  messageId: string,
): Promise<ClientSnapshot> {
  if (!hasTauriRuntime()) {
    return mockBackend.forwardMessage(sourceChatId, targetChatId, messageId);
  }

  return invoke<ClientSnapshot>("forward_message", {
    source_chat_id: sourceChatId,
    target_chat_id: targetChatId,
    message_id: messageId,
  });
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

export async function exportDeviceRegistration(path: string): Promise<void> {
  if (!hasTauriRuntime()) {
    return;
  }

  return invoke<void>("export_device_registration", { path });
}

export async function previewInviteLink(
  inviteLink: string,
): Promise<ClientSnapshot> {
  if (!hasTauriRuntime()) {
    return mockBackend.previewInvite(inviteLink);
  }

  return invoke<ClientSnapshot>("preview_invite", { invite_link: inviteLink });
}

export async function acceptInviteLink(
  inviteLink: string,
): Promise<ClientSnapshot> {
  if (!hasTauriRuntime()) {
    return mockBackend.acceptInvite(inviteLink);
  }

  return invoke<ClientSnapshot>("accept_invite", { invite_link: inviteLink });
}

export async function checkForClientUpdates(): Promise<ClientSnapshot> {
  if (!hasTauriRuntime()) {
    return mockBackend.checkForUpdates();
  }

  return invoke<ClientSnapshot>("check_for_updates");
}

export async function startChatWithPeer(deviceId: string): Promise<ClientSnapshot> {
  if (!hasTauriRuntime()) {
    return mockBackend.startChatWithPeer(deviceId);
  }

  return invoke<ClientSnapshot>("start_chat_with_peer", { device_id: deviceId });
}
