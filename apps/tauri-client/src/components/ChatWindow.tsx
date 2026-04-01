import { useRef } from "react";

import type { CopyBundle } from "../i18n";
import type { ChatThreadView, ThemeMode } from "../types";

type ComposerMessageTarget = {
  messageId: string;
  author: string;
  preview: string;
};

type ChatWindowProps = {
  chat: ChatThreadView | null;
  theme: ThemeMode;
  copy: CopyBundle;
  composerValue: string;
  onComposerChange: (value: string) => void;
  onSend: () => void;
  onSelectMedia: (file: File | null) => void;
  isRecordingVoice: boolean;
  onToggleVoiceRecording: () => void;
  pendingReply: ComposerMessageTarget | null;
  onCancelReply: () => void;
  onReply: (messageId: string) => void;
  onForward: (messageId: string) => void;
  onToggleReaction: (messageId: string, reaction: string) => void;
};

function getAvatarColor(title: string): number {
  let hash = 0;
  for (let i = 0; i < title.length; i++) {
    hash = title.charCodeAt(i) + ((hash << 5) - hash);
  }
  return Math.abs(hash) % 8;
}

function deliveryIcon(
  state: ChatThreadView["messages"][number]["deliveryState"],
): string {
  switch (state) {
    case "sent":
      return "✓";
    case "delivered":
      return "✓✓";
    case "seen":
      return "✓✓";
    case "queued":
    default:
      return "…";
  }
}

const REACTION_OPTIONS = ["👍", "❤️", "🔥"];

export function ChatWindow({
  chat,
  theme,
  copy,
  composerValue,
  onComposerChange,
  onSend,
  onSelectMedia,
  isRecordingVoice,
  onToggleVoiceRecording,
  pendingReply,
  onCancelReply,
  onReply,
  onForward,
  onToggleReaction,
}: ChatWindowProps) {
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const messagesEndRef = useRef<HTMLDivElement | null>(null);

  if (!chat) {
    return (
      <main className="chat-main">
        <div className="chat-bg-pattern" />
        <div className="empty-chat">
          <div className="empty-chat-content">
            <div className="empty-chat-icon">💬</div>
            <p className="empty-chat-text">{copy.noChatSelected}</p>
          </div>
        </div>
      </main>
    );
  }

  const colorIndex = getAvatarColor(chat.title);
  const isOnline = chat.presenceState === "online";

  return (
    <main className="chat-main">
      <div className="chat-bg-pattern" />

      <header className="chat-header">
        <div className="chat-header-left">
          <span className={`avatar avatar-small avatar-color-${colorIndex}`}>
            {chat.title.slice(0, 1)}
          </span>
          <div className="chat-header-info">
            <div className="chat-header-name">{chat.title}</div>
            <div
              className={`chat-header-status${isOnline ? "" : " offline"}`}
            >
              {chat.presenceLabel}
            </div>
          </div>
        </div>
        <div className="chat-header-actions">
          <button className="header-action-btn" type="button" title="Search">
            🔍
          </button>
          <button className="header-action-btn" type="button" title="Call">
            📞
          </button>
          <button className="header-action-btn" type="button" title="More">
            ⋮
          </button>
        </div>
      </header>

      <div className="security-banner">
        🔒 {copy.e2eeBadge} · {chat.securityLabel}
      </div>

      <div className="message-stream">
        <div className="message-list">
          {chat.messages.map((message, index) => {
            const prevMessage = index > 0 ? chat.messages[index - 1] : null;
            const showSender =
              message.direction !== "system" &&
              (!prevMessage || prevMessage.author !== message.author);
            const isOutbound = message.direction === "outbound";

            return (
              <div
                className={`message-bubble-group ${message.direction}`}
                key={message.id}
              >
                <div className="message-bubble-wrapper">
                  <div className="message-hover-actions">
                    <button
                      className="hover-action-btn"
                      onClick={() => onReply(message.id)}
                      title={copy.replyAction}
                      type="button"
                    >
                      ↩
                    </button>
                    <button
                      className="hover-action-btn"
                      onClick={() => onForward(message.id)}
                      title={copy.forwardAction}
                      type="button"
                    >
                      ↪
                    </button>
                    {REACTION_OPTIONS.map((reaction) => (
                      <button
                        className="hover-action-btn"
                        key={`${message.id}-${reaction}`}
                        onClick={() => onToggleReaction(message.id, reaction)}
                        title={reaction}
                        type="button"
                      >
                        {reaction}
                      </button>
                    ))}
                  </div>

                  <div
                    className={`message-bubble ${message.direction} has-tail`}
                  >
                    {showSender && message.direction !== "system" ? (
                      <div className="message-sender-name">
                        {message.author}
                      </div>
                    ) : null}

                    {message.forwardedFrom ? (
                      <div className="message-forwarded">
                        {copy.forwardedFrom}: {message.forwardedFrom}
                      </div>
                    ) : null}

                    {message.replyPreview ? (
                      <div className="message-reply-block">
                        <div className="message-reply-author">
                          {isOutbound ? copy.youLabel : message.author}
                        </div>
                        <div className="message-reply-text">
                          {message.replyPreview}
                        </div>
                      </div>
                    ) : null}

                    <p className="message-text">
                      {message.body}
                      <span className="message-meta">
                        <span className="message-time">
                          {message.timestampLabel}
                        </span>
                        {isOutbound ? (
                          <span
                            className={`message-delivery${message.deliveryState === "queued" ? " pending" : ""}`}
                          >
                            {deliveryIcon(message.deliveryState)}
                          </span>
                        ) : null}
                      </span>
                    </p>

                    {message.attachments.map((attachment) => (
                      <div className="message-attachment" key={attachment.id}>
                        {attachment.previewDataUrl ? (
                          attachment.mimeType.startsWith("image/") ? (
                            <img
                              alt={attachment.fileName}
                              className="attachment-image"
                              src={attachment.previewDataUrl}
                            />
                          ) : attachment.mimeType.startsWith("audio/") ? (
                            <audio
                              className="attachment-audio"
                              controls
                              preload="metadata"
                              src={attachment.previewDataUrl}
                            />
                          ) : attachment.mimeType === "application/pdf" ? (
                            <div className="attachment-file">
                              <span className="attachment-file-icon">📄</span>
                              <div className="attachment-file-info">
                                <div className="attachment-file-name">
                                  {attachment.fileName}
                                </div>
                                <div className="attachment-file-size">
                                  {attachment.sizeLabel}
                                </div>
                              </div>
                            </div>
                          ) : (
                            <div className="attachment-file">
                              <span className="attachment-file-icon">📎</span>
                              <div className="attachment-file-info">
                                <div className="attachment-file-name">
                                  {attachment.fileName}
                                </div>
                                <div className="attachment-file-size">
                                  {attachment.sizeLabel}
                                </div>
                              </div>
                            </div>
                          )
                        ) : (
                          <div className="attachment-file">
                            <span className="attachment-file-icon">📎</span>
                            <div className="attachment-file-info">
                              <div className="attachment-file-name">
                                {attachment.fileName}
                              </div>
                              <div className="attachment-file-size">
                                {attachment.sizeLabel}
                              </div>
                            </div>
                          </div>
                        )}
                      </div>
                    ))}

                    {message.reactions.length > 0 ? (
                      <div className="message-reactions">
                        {message.reactions.map((reaction) => (
                          <button
                            className="reaction-chip is-active"
                            key={`${message.id}-r-${reaction}`}
                            onClick={() =>
                              onToggleReaction(message.id, reaction)
                            }
                            type="button"
                          >
                            {reaction}
                          </button>
                        ))}
                      </div>
                    ) : null}
                  </div>
                </div>
              </div>
            );
          })}
          <div ref={messagesEndRef} />
        </div>
      </div>

      <div className="composer">
        {pendingReply ? (
          <div className="composer-reply-bar">
            <div className="composer-reply-content">
              <div className="composer-reply-author">
                {pendingReply.author}
              </div>
              <div className="composer-reply-text">
                {pendingReply.preview}
              </div>
            </div>
            <button
              className="composer-reply-close"
              onClick={onCancelReply}
              type="button"
            >
              ✕
            </button>
          </div>
        ) : null}

        <div className="composer-inner">
          <button
            className="composer-btn"
            onClick={() => fileInputRef.current?.click()}
            type="button"
            title={copy.attachMedia}
          >
            📎
          </button>

          <div className="composer-input-wrapper">
            <textarea
              aria-label={copy.composerPlaceholder}
              className="composer-input"
              onChange={(event) => onComposerChange(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter" && !event.shiftKey) {
                  event.preventDefault();
                  onSend();
                }
              }}
              placeholder={copy.composerPlaceholder}
              rows={1}
              value={composerValue}
            />
          </div>

          <button
            className="composer-btn"
            type="button"
            title="Emoji"
          >
            😊
          </button>

          {composerValue.trim().length > 0 ? (
            <button
              className="composer-send-btn has-text"
              onClick={onSend}
              type="button"
              title={copy.send}
            >
              ➤
            </button>
          ) : (
            <button
              className={`composer-send-btn${isRecordingVoice ? "" : ""}`}
              onClick={onToggleVoiceRecording}
              type="button"
              title={isRecordingVoice ? copy.stopVoiceRecording : copy.recordVoice}
              style={
                isRecordingVoice
                  ? { color: "#e17076" }
                  : undefined
              }
            >
              🎤
            </button>
          )}
        </div>

        <input
          className="hidden-file-input"
          onChange={(event) => {
            onSelectMedia(event.target.files?.[0] ?? null);
            event.target.value = "";
          }}
          ref={fileInputRef}
          type="file"
        />
      </div>
    </main>
  );
}
