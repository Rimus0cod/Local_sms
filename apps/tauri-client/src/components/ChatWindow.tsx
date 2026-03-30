import type { CopyBundle } from "../i18n";
import type { ChatThreadView } from "../types";

type ChatWindowProps = {
  chat: ChatThreadView | null;
  copy: CopyBundle;
  composerValue: string;
  onComposerChange: (value: string) => void;
  onSend: () => void;
};

export function ChatWindow({
  chat,
  copy,
  composerValue,
  onComposerChange,
  onSend,
}: ChatWindowProps) {
  if (!chat) {
    return (
      <section className="panel panel-chat empty-state">
        <p className="empty-copy">{copy.noChatSelected}</p>
      </section>
    );
  }

  return (
    <section className="panel panel-chat">
      <header className="chat-window-header">
        <div>
          <p className="eyebrow">{chat.securityLabel}</p>
          <h2>{chat.title}</h2>
          <p className="chat-presence">{chat.presenceLabel}</p>
        </div>
        <div className="chat-window-actions">
          <span className="meta-pill">{chat.kind}</span>
          <span className="meta-pill">{chat.participants.length} peers</span>
          <button className="icon-button" type="button">
            ⌕
          </button>
          <button className="icon-button" type="button">
            ☎
          </button>
          <button className="icon-button" type="button">
            ⋯
          </button>
        </div>
      </header>

      <div className="security-banner">
        <strong>{copy.participants}:</strong> {chat.participants.join(", ")}
      </div>

      <div className="message-stream">
        {chat.messages.length === 0 ? (
          <p className="empty-copy">{copy.noMessages}</p>
        ) : (
          chat.messages.map((message) => (
            <article
              className={`message-bubble direction-${message.direction}`}
              key={message.id}
            >
              <div className="message-topline">
                <span>{message.author}</span>
                <span>{copy.messageDirections[message.direction]}</span>
              </div>
              {message.replyPreview ? (
                <div className="reply-preview">{message.replyPreview}</div>
              ) : null}
              <p className="message-body">{message.body}</p>
              <div className="message-footer">
                <span>{message.timestampLabel}</span>
                <span>{copy.deliveryStates[message.deliveryState]}</span>
                {message.reactions.length > 0 ? (
                  <span>{message.reactions.join("  ")}</span>
                ) : null}
              </div>
            </article>
          ))
        )}
      </div>

      <div className="composer">
        <textarea
          aria-label={copy.composerPlaceholder}
          className="composer-input"
          onChange={(event) => onComposerChange(event.target.value)}
          placeholder={copy.composerPlaceholder}
          rows={4}
          value={composerValue}
        />
        <button className="primary-button" onClick={onSend} type="button">
          {copy.send}
        </button>
      </div>
    </section>
  );
}
