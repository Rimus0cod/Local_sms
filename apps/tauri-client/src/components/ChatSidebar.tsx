import type { CopyBundle } from "../i18n";
import type { ChatThreadView } from "../types";

type ChatSidebarProps = {
  chats: ChatThreadView[];
  selectedChatId: string | null;
  copy: CopyBundle;
  onSelect: (chatId: string) => void;
};

export function ChatSidebar({
  chats,
  selectedChatId,
  copy,
  onSelect,
}: ChatSidebarProps) {
  return (
    <aside className="panel panel-sidebar">
      <div className="panel-header">
        <div>
          <p className="eyebrow">{copy.chats}</p>
          <h2>{copy.secureRooms}</h2>
        </div>
      </div>
      <div className="sidebar-search">
        <span className="sidebar-search-glyph">⌕</span>
        <input
          aria-label="Search chats"
          className="sidebar-search-input"
          placeholder="Search"
          type="text"
        />
      </div>
      <div className="chat-list">
        {chats.map((chat) => {
          const active = chat.id === selectedChatId;
          return (
            <button
              key={chat.id}
              className={`chat-card${active ? " is-active" : ""}`}
              onClick={() => onSelect(chat.id)}
              type="button"
            >
              <div className="chat-card-topline">
                <div className="chat-card-identity">
                  <span className="chat-avatar">{chat.title.slice(0, 1)}</span>
                  <span className="chat-title">{chat.title}</span>
                </div>
                {chat.unreadCount > 0 ? (
                  <span className="chat-unread">{chat.unreadCount}</span>
                ) : null}
              </div>
              <p className="chat-summary">{chat.summary}</p>
              <div className="chat-meta-row">
                <span className="meta-pill">{chat.kind}</span>
                <span className="meta-pill">{chat.securityLabel}</span>
              </div>
              <div className="chat-meta-row">
                <span>{chat.presenceLabel}</span>
                <span>
                  {chat.messages[chat.messages.length - 1]?.timestampLabel ?? "--"}
                </span>
              </div>
            </button>
          );
        })}
      </div>
    </aside>
  );
}
