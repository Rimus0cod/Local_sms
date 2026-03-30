import { useEffect, useState } from "react";

import { ChatSidebar } from "./components/ChatSidebar";
import { ChatWindow } from "./components/ChatWindow";
import { PeerDiscoveryPanel } from "./components/PeerDiscoveryPanel";
import { VerificationPanel } from "./components/VerificationPanel";
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
  const refreshPeers = useClientStore((state) => state.refreshPeers);
  const sendMessage = useClientStore((state) => state.sendMessage);
  const verifyDevice = useClientStore((state) => state.verifyDevice);
  const toggleTheme = useClientStore((state) => state.toggleTheme);
  const setLocale = useClientStore((state) => state.setLocale);

  const [composerValue, setComposerValue] = useState("");

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    setComposerValue("");
  }, [selectedChatId]);

  const copy = COPY[locale];
  const selectedChat =
    snapshot?.chats.find((chat) => chat.id === selectedChatId) ?? null;

  return (
    <main className="app-shell" data-theme={theme}>
      <div className="signal-grid" />
      <div className="signal-noise" />
      <div className="ambient-orb ambient-left" />
      <div className="ambient-orb ambient-right" />

      <header className="topbar">
        <div className="topbar-copy">
          <p className="eyebrow">{copy.title}</p>
          <h1>{copy.subtitle}</h1>
          <p className="topbar-note">
            Local-only secure relay for a trusted mesh.
          </p>
        </div>
        <div className="topbar-controls">
          <span className="badge">{copy.lanBadge}</span>
          <span className="badge accent">{copy.e2eeBadge}</span>
          <button className="secondary-button" onClick={toggleTheme} type="button">
            {copy.themeLabel}: {theme}
          </button>
          <div className="locale-switch">
            <span>{copy.localeLabel}</span>
            <button
              className={`locale-button${locale === "ru" ? " is-active" : ""}`}
              onClick={() => setLocale("ru")}
              type="button"
            >
              RU
            </button>
            <button
              className={`locale-button${locale === "en" ? " is-active" : ""}`}
              onClick={() => setLocale("en")}
              type="button"
            >
              EN
            </button>
          </div>
        </div>
      </header>

      {error ? <div className="error-banner">{error}</div> : null}

      <section className="status-strip">
        <div className="status-node">
          <span>{copy.operatorLabel}</span>
          <strong>{snapshot?.localProfile.displayName ?? "..."}</strong>
        </div>
        <div className="status-node">
          <span>{copy.trustedLocalDevicesLabel}</span>
          <strong>
            {snapshot?.localProfile.trustedDeviceCount ?? 0}/
            {snapshot?.localProfile.totalDeviceCount ?? 0}
          </strong>
        </div>
        <div className="status-node">
          <span>{copy.discoveryScopeLabel}</span>
          <strong>{snapshot?.transportStatus.discoveryMode ?? "mDNS"}</strong>
        </div>
      </section>

      <section className="workspace">
        <aside className="utility-rail">
          <button className="rail-button is-active" type="button">
            <span className="rail-glyph">≡</span>
            <span className="rail-label">Chats</span>
          </button>
          <button className="rail-button" type="button">
            <span className="rail-glyph">◌</span>
            <span className="rail-label">Mesh</span>
          </button>
          <button className="rail-button" type="button">
            <span className="rail-glyph">⌘</span>
            <span className="rail-label">Trust</span>
          </button>
        </aside>

        <section className="main-grid">
        <ChatSidebar
          chats={snapshot?.chats ?? []}
          copy={copy}
          selectedChatId={selectedChatId}
          onSelect={selectChat}
        />

        <ChatWindow
          chat={selectedChat}
          composerValue={composerValue}
          copy={copy}
          onComposerChange={setComposerValue}
          onSend={() => {
            if (composerValue.trim().length === 0) {
              return;
            }

            void sendMessage(composerValue);
            setComposerValue("");
          }}
        />

        <div className="context-stack">
          <PeerDiscoveryPanel
            busy={busy}
            copy={copy}
            onRefresh={() => void refreshPeers()}
            peerStates={copy.peerStates}
            peers={snapshot?.peers ?? []}
            refreshLabel={copy.refreshPeers}
            transportStatus={
              snapshot?.transportStatus ?? {
                discoveryMode: "mDNS peer discovery",
                transportMode: "QUIC transport",
                cryptoMode: "X3DH bootstrap + Double Ratchet",
                storageMode: "Encrypted SQLite at rest",
              }
            }
          />

          <VerificationPanel
            busy={busy}
            copy={copy}
            devices={snapshot?.verification.devices ?? []}
            onSelect={selectVerificationDevice}
            onVerifyQr={() => void verifyDevice("qr")}
            onVerifySafety={() => void verifyDevice("safety")}
            pendingDeviceCount={snapshot?.verification.pendingDeviceCount ?? 0}
            selectedDeviceId={selectedVerificationDeviceId}
            trustedDeviceCount={snapshot?.verification.trustedDeviceCount ?? 0}
          />
        </div>
        </section>
      </section>
    </main>
  );
}
