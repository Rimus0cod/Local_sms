import type { CopyBundle } from "../i18n";
import type { ChatThreadView, LocalProfileView, LocaleCode, ThemeMode } from "../types";

type ChatSidebarProps = {
  chats: ChatThreadView[];
  selectedChatId: string | null;
  locale: LocaleCode;
  theme: ThemeMode;
  copy: CopyBundle;
  localProfile: LocalProfileView | null;
  onSelect: (chatId: string) => void;
  onToggleTheme: () => void;
  onSetLocale: (locale: LocaleCode) => void;
  onOpenSettings: () => void;
  onOpenInvite: () => void;
};

function getAvatarColor(title: string): number {
  let hash = 0;
  for (let i = 0; i < title.length; i++) {
    hash = title.charCodeAt(i) + ((hash << 5) - hash);
  }
  return Math.abs(hash) % 8;
}

export function ChatSidebar({
  chats,
  selectedChatId,
  locale,
  theme,
  copy,
  localProfile,
  onSelect,
  onToggleTheme,
  onSetLocale,
  onOpenSettings,
  onOpenInvite,
}: ChatSidebarProps) {
  return (
    <aside className="sidebar">
      <div className="sidebar-header">
        <button
          className="hamburger-btn"
          onClick={onOpenSettings}
          type="button"
          title={copy.settingsTitle}
        >
          ☰
        </button>
        <div className="sidebar-search-wrapper">
          <span className="search-icon">🔍</span>
          <input
            aria-label="Search"
            className="sidebar-search"
            placeholder={copy.searchPlaceholder}
            type="text"
          />
        </div>
        <button
          className="theme-toggle-btn"
          onClick={onToggleTheme}
          type="button"
          title={theme === "midnight" ? copy.themeLight : copy.themeDark}
        >
          {theme === "midnight" ? "☀" : "🌙"}
        </button>
      </div>

      <div className="chat-list">
        {chats.length === 0 ? (
          <div className="sidebar-empty-state">
            <div className="sidebar-empty-icon">👋</div>
            <p className="sidebar-empty-title">{copy.inviteEmptyTitle}</p>
            <p className="sidebar-empty-subtitle">{copy.inviteEmptySubtitle}</p>
          </div>
        ) : chats.map((chat) => {
          const active = chat.id === selectedChatId;
          const lastMessage = chat.messages[chat.messages.length - 1];
          const lastTimestamp = lastMessage?.timestampLabel ?? "";
          const colorIndex = getAvatarColor(chat.title);
          const senderPrefix =
            lastMessage && lastMessage.direction !== "system"
              ? lastMessage.direction === "outbound"
                ? `${copy.youLabel}: `
                : `${lastMessage.author}: `
              : "";

          return (
            <button
              key={chat.id}
              className={`chat-item${active ? " is-active" : ""}`}
              onClick={() => onSelect(chat.id)}
              type="button"
            >
              <span className={`avatar avatar-color-${colorIndex}`}>
                {chat.title.slice(0, 1)}
              </span>
              <div className="chat-content">
                <div className="chat-top-row">
                  <span className="chat-name">{chat.title}</span>
                  <span className="chat-timestamp">{lastTimestamp}</span>
                </div>
                <div className="chat-bottom-row">
                  <span className="chat-last-message">
                    {senderPrefix ? (
                      <span className="chat-sender">{senderPrefix}</span>
                    ) : null}
                    {chat.summary}
                  </span>
                  {chat.unreadCount > 0 ? (
                    <span className="unread-badge">{chat.unreadCount}</span>
                  ) : null}
                </div>
              </div>
            </button>
          );
        })}
      </div>

      <div className="sidebar-invite-bar">
        <button
          className="sidebar-invite-btn"
          onClick={onOpenInvite}
          type="button"
        >
          <span className="sidebar-invite-icon">＋</span>
          <span>{copy.inviteFriend}</span>
        </button>
      </div>

      <div className="sidebar-bottom">
        <button
          className="sidebar-bottom-btn"
          onClick={onOpenSettings}
          type="button"
        >
          <span className="avatar avatar-tiny avatar-color-3">
            {localProfile?.displayName?.slice(0, 1) ?? "?"}
          </span>
          <div className="sidebar-bottom-info">
            <span className="sidebar-bottom-name">
              {localProfile?.displayName ?? "..."}
            </span>
            <span className="sidebar-bottom-status">
              {localProfile?.trustedDeviceCount ?? 0}/{localProfile?.totalDeviceCount ?? 0} {copy.settingsVerifiedLabel}
            </span>
          </div>
          <span className="sidebar-bottom-gear">⚙</span>
        </button>
      </div>
    </aside>
  );
}
