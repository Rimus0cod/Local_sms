import { useEffect, useRef, useState, type MutableRefObject } from "react";

import { ChatSidebar } from "./components/ChatSidebar";
import { ChatWindow } from "./components/ChatWindow";
import { InvitePanel } from "./components/InvitePanel";
import { SettingsPanel } from "./components/SettingsPanel";
import { COPY } from "./i18n";
import { useClientStore } from "./store/clientStore";

export function App() {
  const snapshot = useClientStore((state) => state.snapshot);
  const selectedChatId = useClientStore((state) => state.selectedChatId);
  const selectedVerificationDeviceId = useClientStore(
    (state) => state.selectedVerificationDeviceId,
  );
  const locale = useClientStore((state) => state.locale);
  const theme = useClientStore((state) => state.theme);
  const busy = useClientStore((state) => state.busy);
  const error = useClientStore((state) => state.error);
  const load = useClientStore((state) => state.load);
  const selectChat = useClientStore((state) => state.selectChat);
  const selectVerificationDevice = useClientStore(
    (state) => state.selectVerificationDevice,
  );
  const sendMessage = useClientStore((state) => state.sendMessage);
  const sendReply = useClientStore((state) => state.sendReply);
  const sendMedia = useClientStore((state) => state.sendMedia);
  const toggleReaction = useClientStore((state) => state.toggleReaction);
  const forwardMessage = useClientStore((state) => state.forwardMessage);
  const refreshPeers = useClientStore((state) => state.refreshPeers);
  const verifyDevice = useClientStore((state) => state.verifyDevice);
  const previewInvite = useClientStore((state) => state.previewInvite);
  const acceptInvite = useClientStore((state) => state.acceptInvite);
  const createContactInvite = useClientStore((state) => state.createContactInvite);
  const previewContactInvite = useClientStore((state) => state.previewContactInvite);
  const acceptContactInvite = useClientStore((state) => state.acceptContactInvite);
  const checkForUpdates = useClientStore((state) => state.checkForUpdates);
  const toggleTheme = useClientStore((state) => state.toggleTheme);
  const setLocale = useClientStore((state) => state.setLocale);

  const [composerValue, setComposerValue] = useState("");
  const [pendingReply, setPendingReply] = useState<{
    messageId: string;
    author: string;
    preview: string;
  } | null>(null);
  const [isRecordingVoice, setIsRecordingVoice] = useState(false);
  const [voiceStatus, setVoiceStatus] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [inviteOpen, setInviteOpen] = useState(false);
  const [inviteDraft, setInviteDraft] = useState("");
  const [contactInviteDraft, setContactInviteDraft] = useState("");
  const [generatedContactInvite, setGeneratedContactInvite] = useState("");
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const mediaStreamRef = useRef<MediaStream | null>(null);
  const voiceChunksRef = useRef<Blob[]>([]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    setComposerValue("");
    setPendingReply(null);
  }, [selectedChatId]);

  const copy = COPY[locale];
  const selectedChat =
    snapshot?.chats.find((chat) => chat.id === selectedChatId) ?? null;

  return (
    <div className="app" data-theme={theme}>
      <ChatSidebar
        chats={snapshot?.chats ?? []}
        selectedChatId={selectedChatId}
        locale={locale}
        theme={theme}
        copy={copy}
        localProfile={snapshot?.localProfile ?? null}
        onSelect={selectChat}
        onToggleTheme={toggleTheme}
        onSetLocale={setLocale}
        onOpenSettings={() => setSettingsOpen(true)}
        onOpenInvite={() => setInviteOpen(true)}
      />

      <ChatWindow
        chat={selectedChat}
        theme={theme}
        copy={copy}
        composerValue={composerValue}
        onComposerChange={setComposerValue}
        onSend={() => {
          if (composerValue.trim().length === 0) {
            return;
          }

          if (pendingReply) {
            void sendReply(pendingReply.messageId, composerValue);
            setPendingReply(null);
          } else {
            void sendMessage(composerValue);
          }
          setComposerValue("");
        }}
        onSelectMedia={(file) => {
          if (!file) {
            return;
          }

          void fileToBase64(file).then((bytesBase64) =>
            sendMedia(
              file.name,
              file.type || "application/octet-stream",
              bytesBase64,
              pendingReply?.messageId ?? null,
            ),
          );
          setPendingReply(null);
        }}
        isRecordingVoice={isRecordingVoice}
        onToggleVoiceRecording={() => {
          void toggleVoiceRecording({
            isRecordingVoice,
            mediaRecorderRef,
            mediaStreamRef,
            voiceChunksRef,
            sendMedia,
            setIsRecordingVoice,
            setVoiceStatus,
          });
        }}
        pendingReply={pendingReply}
        onCancelReply={() => setPendingReply(null)}
        onReply={(messageId) => {
          const message = selectedChat?.messages.find(
            (entry) => entry.id === messageId,
          );
          if (!message) {
            return;
          }
          setPendingReply({
            messageId,
            author: message.author,
            preview: message.body,
          });
        }}
        onForward={(messageId) => {
          if (!selectedChatId || !selectedChat) {
            return;
          }
          const message = selectedChat.messages.find(
            (entry) => entry.id === messageId,
          );
          if (!message) {
            return;
          }
          void forwardMessage(selectedChatId, messageId);
        }}
        onToggleReaction={(messageId, reaction) => {
          void toggleReaction(messageId, reaction);
        }}
      />

      <SettingsPanel
        open={settingsOpen}
        snapshot={snapshot}
        theme={theme}
        locale={locale}
        copy={copy}
        busy={busy}
        inviteDraft={inviteDraft}
        contactInviteDraft={contactInviteDraft}
        generatedContactInvite={generatedContactInvite}
        selectedVerificationDeviceId={selectedVerificationDeviceId}
        onClose={() => setSettingsOpen(false)}
        onToggleTheme={toggleTheme}
        onSetLocale={setLocale}
        onRefreshPeers={() => void refreshPeers()}
        onStartChatWithPeer={(deviceId) => void useClientStore.getState().startChatWithPeer(deviceId)}
        onVerifyDevice={(action) => void verifyDevice(action)}
        onSelectVerificationDevice={selectVerificationDevice}
        onInviteDraftChange={setInviteDraft}
        onPreviewInvite={() => void previewInvite(inviteDraft)}
        onAcceptInvite={() => void acceptInvite(inviteDraft)}
        onContactInviteDraftChange={setContactInviteDraft}
        onCreateContactInvite={() => {
          void createContactInvite().then((invite) => {
            setGeneratedContactInvite(invite);
          });
        }}
        onPreviewContactInvite={() => void previewContactInvite(contactInviteDraft)}
        onAcceptContactInvite={() => void acceptContactInvite(contactInviteDraft)}
        onCheckForUpdates={() => void checkForUpdates()}
      />

      {inviteOpen ? (
        <InvitePanel
          copy={copy}
          inviteLink={snapshot?.transportStatus.serverStatus !== "not configured"
            ? `localmessenger://join/${snapshot?.localProfile.activeDeviceId ?? "device"}`
            : `localmessenger://lan/${snapshot?.localProfile.activeDeviceId ?? "device"}`}
          onClose={() => setInviteOpen(false)}
        />
      ) : null}

      {error ? (
        <div
          style={{
            position: "fixed",
            bottom: 16,
            left: "50%",
            transform: "translateX(-50%)",
            padding: "8px 16px",
            borderRadius: 8,
            background: "rgba(220, 50, 50, 0.9)",
            color: "#fff",
            fontSize: 13,
            zIndex: 100,
          }}
        >
          {error}
        </div>
      ) : null}
      {voiceStatus ? (
        <div
          style={{
            position: "fixed",
            bottom: error ? 52 : 16,
            left: "50%",
            transform: "translateX(-50%)",
            padding: "8px 16px",
            borderRadius: 8,
            background: "rgba(94, 168, 222, 0.9)",
            color: "#fff",
            fontSize: 13,
            zIndex: 100,
          }}
        >
          {voiceStatus}
        </div>
      ) : null}
    </div>
  );
}

async function fileToBase64(file: File): Promise<string> {
  const buffer = await file.arrayBuffer();
  let binary = "";
  const bytes = new Uint8Array(buffer);
  bytes.forEach((value) => {
    binary += String.fromCharCode(value);
  });
  return btoa(binary);
}

async function toggleVoiceRecording({
  isRecordingVoice,
  mediaRecorderRef,
  mediaStreamRef,
  voiceChunksRef,
  sendMedia,
  setIsRecordingVoice,
  setVoiceStatus,
}: {
  isRecordingVoice: boolean;
  mediaRecorderRef: MutableRefObject<MediaRecorder | null>;
  mediaStreamRef: MutableRefObject<MediaStream | null>;
  voiceChunksRef: MutableRefObject<Blob[]>;
  sendMedia: (
    fileName: string,
    mimeType: string,
    bytesBase64: string,
  ) => Promise<void>;
  setIsRecordingVoice: (value: boolean) => void;
  setVoiceStatus: (value: string | null) => void;
}) {
  if (isRecordingVoice) {
    mediaRecorderRef.current?.stop();
    return;
  }

  if (
    typeof navigator === "undefined" ||
    !navigator.mediaDevices ||
    typeof MediaRecorder === "undefined"
  ) {
    setVoiceStatus("Voice recording is not supported in this runtime.");
    return;
  }

  try {
    const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
    const mimeType = MediaRecorder.isTypeSupported(
      "audio/webm;codecs=opus",
    )
      ? "audio/webm;codecs=opus"
      : "audio/webm";
    const recorder = new MediaRecorder(stream, { mimeType });
    voiceChunksRef.current = [];
    mediaStreamRef.current = stream;
    mediaRecorderRef.current = recorder;
    recorder.ondataavailable = (event) => {
      if (event.data.size > 0) {
        voiceChunksRef.current.push(event.data);
      }
    };
    recorder.onerror = () => {
      setVoiceStatus("Voice recording failed.");
      setIsRecordingVoice(false);
    };
    recorder.onstop = () => {
      const blob = new Blob(voiceChunksRef.current, {
        type: recorder.mimeType || "audio/webm",
      });
      mediaStreamRef.current?.getTracks().forEach((track) => track.stop());
      mediaStreamRef.current = null;
      mediaRecorderRef.current = null;
      setIsRecordingVoice(false);
      if (blob.size === 0) {
        setVoiceStatus("Voice note was empty.");
        return;
      }
      void blobToBase64(blob)
        .then((bytesBase64) =>
          sendMedia(
            `voice-note-${Date.now()}.webm`,
            blob.type || "audio/webm",
            bytesBase64,
          ),
        )
        .then(() => setVoiceStatus(null))
        .catch((recordingError: unknown) => {
          setVoiceStatus(
            recordingError instanceof Error
              ? recordingError.message
              : "Voice message send failed.",
          );
        });
    };
    recorder.start();
    setVoiceStatus("Recording voice note...");
    setIsRecordingVoice(true);
  } catch (recordingError) {
    setVoiceStatus(
      recordingError instanceof Error
        ? recordingError.message
        : "Unable to access microphone.",
    );
  }
}

async function blobToBase64(blob: Blob): Promise<string> {
  const buffer = await blob.arrayBuffer();
  let binary = "";
  new Uint8Array(buffer).forEach((value) => {
    binary += String.fromCharCode(value);
  });
  return btoa(binary);
}
