import { useState } from "react";

import type { CopyBundle } from "../i18n";
import type {
  ClientSnapshot,
  LocaleCode,
  ThemeMode,
  VerificationDeviceView,
} from "../types";

type SettingsPanelProps = {
  open: boolean;
  snapshot: ClientSnapshot | null;
  theme: ThemeMode;
  locale: LocaleCode;
  copy: CopyBundle;
  busy: boolean;
  inviteDraft: string;
  contactInviteDraft: string;
  generatedContactInvite: string;
  selectedVerificationDeviceId: string | null;
  onClose: () => void;
  onToggleTheme: () => void;
  onSetLocale: (locale: LocaleCode) => void;
  onRefreshPeers: () => void;
  onStartChatWithPeer: (deviceId: string) => void;
  onVerifyDevice: (action: "qr" | "safety") => void;
  onSelectVerificationDevice: (deviceId: string) => void;
  onInviteDraftChange: (value: string) => void;
  onPreviewInvite: () => void;
  onAcceptInvite: () => void;
  onContactInviteDraftChange: (value: string) => void;
  onCreateContactInvite: () => void;
  onPreviewContactInvite: () => void;
  onAcceptContactInvite: () => void;
  onCheckForUpdates: () => void;
};

type SettingsSection = "main" | "general" | "devices" | "network" | "relay" | "updates";

export function SettingsPanel({
  open,
  snapshot,
  theme,
  locale,
  copy,
  busy,
  inviteDraft,
  contactInviteDraft,
  generatedContactInvite,
  selectedVerificationDeviceId,
  onClose,
  onToggleTheme,
  onSetLocale,
  onRefreshPeers,
  onStartChatWithPeer,
  onVerifyDevice,
  onSelectVerificationDevice,
  onInviteDraftChange,
  onPreviewInvite,
  onAcceptInvite,
  onContactInviteDraftChange,
  onCreateContactInvite,
  onPreviewContactInvite,
  onAcceptContactInvite,
  onCheckForUpdates,
}: SettingsPanelProps) {
  const [section, setSection] = useState<SettingsSection>("main");

  if (!open) {
    return null;
  }

  const profile = snapshot?.localProfile ?? null;
  const transport = snapshot?.transportStatus ?? null;
  const peers = snapshot?.peers ?? [];
  const onlinePeers = peers.filter((p) => p.state === "live").length;
  const devices = snapshot?.verification.devices ?? [];
  const trustedCount = snapshot?.verification.trustedDeviceCount ?? 0;
  const pendingCount = snapshot?.verification.pendingDeviceCount ?? 0;
  const updater = snapshot?.updater ?? null;
  const selectedDevice =
    devices.find((d) => d.deviceId === selectedVerificationDeviceId) ?? null;

  return (
    <div className="settings-overlay" onClick={onClose}>
      <div
        className="settings-panel"
        onClick={(e) => e.stopPropagation()}
      >
        {section === "main" ? (
          <SettingsMain
            copy={copy}
            profile={profile}
            theme={theme}
            activeRoute={transport?.activeRoute ?? "direct_lan"}
            peersOnline={onlinePeers}
            peersTotal={peers.length}
            trustedCount={trustedCount}
            pendingCount={pendingCount}
            updaterVersion={updater?.currentVersion ?? "—"}
            onClose={onClose}
            onNavigate={setSection}
          />
        ) : null}

        {section === "general" ? (
          <SettingsGeneral
            copy={copy}
            theme={theme}
            locale={locale}
            onBack={() => setSection("main")}
            onToggleTheme={onToggleTheme}
            onSetLocale={onSetLocale}
          />
        ) : null}

        {section === "devices" ? (
          <SettingsDevices
            copy={copy}
            devices={devices}
            selectedDeviceId={selectedVerificationDeviceId}
            trustedCount={trustedCount}
            pendingCount={pendingCount}
            busy={busy}
            onBack={() => setSection("main")}
            onSelect={onSelectVerificationDevice}
            onVerifyQr={() => onVerifyDevice("qr")}
            onVerifySafety={() => onVerifyDevice("safety")}
          />
        ) : null}

        {section === "network" ? (
          <SettingsNetwork
            copy={copy}
            transport={transport}
            peers={peers}
            peerStates={copy.peerStates}
            busy={busy}
            onBack={() => setSection("main")}
            onRefresh={onRefreshPeers}
            onStartChat={onStartChatWithPeer}
          />
        ) : null}

        {section === "relay" ? (
          <SettingsRelay
            copy={copy}
            inviteDraft={inviteDraft}
            contactInviteDraft={contactInviteDraft}
            generatedContactInvite={generatedContactInvite}
            busy={busy}
            onBack={() => setSection("main")}
            onInviteDraftChange={onInviteDraftChange}
            onPreview={onPreviewInvite}
            onAccept={onAcceptInvite}
            onContactInviteDraftChange={onContactInviteDraftChange}
            onCreateContactInvite={onCreateContactInvite}
            onPreviewContactInvite={onPreviewContactInvite}
            onAcceptContactInvite={onAcceptContactInvite}
          />
        ) : null}

        {section === "updates" ? (
          <SettingsUpdates
            copy={copy}
            updater={updater}
            busy={busy}
            onBack={() => setSection("main")}
            onCheck={onCheckForUpdates}
          />
        ) : null}
      </div>
    </div>
  );
}

/* ── Main menu ────────────────────────────────────────── */

function SettingsMain({
  copy,
  profile,
  theme,
  activeRoute,
  peersOnline,
  peersTotal,
  trustedCount,
  pendingCount,
  updaterVersion,
  onClose,
  onNavigate,
}: {
  copy: CopyBundle;
  profile: ClientSnapshot["localProfile"] | null;
  theme: ThemeMode;
  activeRoute: string;
  peersOnline: number;
  peersTotal: number;
  trustedCount: number;
  pendingCount: number;
  updaterVersion: string;
  onClose: () => void;
  onNavigate: (s: SettingsSection) => void;
}) {
  return (
    <>
      <div className="settings-header">
        <button className="settings-back-btn" onClick={onClose} type="button">
          ✕
        </button>
        <h2 className="settings-title">{copy.settingsTitle}</h2>
      </div>

      <div className="settings-body">
        <div className="settings-profile-card">
          <span className="avatar avatar-color-3">
            {profile?.displayName?.slice(0, 1) ?? "?"}
          </span>
          <div className="settings-profile-info">
            <div className="settings-profile-name">
              {profile?.displayName ?? "..."}
            </div>
            <div className="settings-profile-device">
              {profile?.activeDeviceName ?? ""}
            </div>
          </div>
        </div>

        <div className="settings-section-list">
          <button
            className="settings-menu-item"
            onClick={() => onNavigate("general")}
            type="button"
          >
            <span className="settings-menu-icon">⚙</span>
            <span className="settings-menu-label">{copy.settingsGeneral}</span>
            <span className="settings-menu-sub">
              {theme === "midnight" ? copy.themeDark : copy.themeLight}
            </span>
            <span className="settings-menu-arrow">›</span>
          </button>

          <button
            className="settings-menu-item"
            onClick={() => onNavigate("devices")}
            type="button"
          >
            <span className="settings-menu-icon">🔐</span>
            <span className="settings-menu-label">{copy.settingsDevices}</span>
            <span className="settings-menu-sub">
              {trustedCount} {copy.settingsVerifiedLabel} · {pendingCount} {copy.settingsPendingLabel}
            </span>
            <span className="settings-menu-arrow">›</span>
          </button>

          <button
            className="settings-menu-item"
            onClick={() => onNavigate("network")}
            type="button"
          >
            <span className="settings-menu-icon">🌐</span>
            <span className="settings-menu-label">{copy.settingsNetwork}</span>
            <span className="settings-menu-sub">
              {peersOnline}/{peersTotal} {copy.settingsOnlineLabel}
            </span>
            <span className="settings-menu-arrow">›</span>
          </button>

          <button
            className="settings-menu-item"
            onClick={() => onNavigate("relay")}
            type="button"
          >
            <span className="settings-menu-icon">📡</span>
            <span className="settings-menu-label">{copy.settingsRelay}</span>
            <span className="settings-menu-sub">
              {activeRoute}
            </span>
            <span className="settings-menu-arrow">›</span>
          </button>

          <button
            className="settings-menu-item"
            onClick={() => onNavigate("updates")}
            type="button"
          >
            <span className="settings-menu-icon">🔄</span>
            <span className="settings-menu-label">{copy.settingsUpdates}</span>
            <span className="settings-menu-sub">
              v{updaterVersion}
            </span>
            <span className="settings-menu-arrow">›</span>
          </button>
        </div>
      </div>
    </>
  );
}

/* ── General settings ─────────────────────────────────── */

function SettingsGeneral({
  copy,
  theme,
  locale,
  onBack,
  onToggleTheme,
  onSetLocale,
}: {
  copy: CopyBundle;
  theme: ThemeMode;
  locale: LocaleCode;
  onBack: () => void;
  onToggleTheme: () => void;
  onSetLocale: (l: LocaleCode) => void;
}) {
  return (
    <>
      <div className="settings-header">
        <button className="settings-back-btn" onClick={onBack} type="button">
          ‹
        </button>
        <h2 className="settings-title">{copy.settingsGeneral}</h2>
      </div>
      <div className="settings-body">
        <div className="settings-group">
          <div className="settings-group-label">{copy.settingsAppearance}</div>
          <button
            className="settings-row"
            onClick={onToggleTheme}
            type="button"
          >
            <span className="settings-row-label">{copy.settingsTheme}</span>
            <span className="settings-row-value">
              {theme === "midnight" ? copy.themeDark : copy.themeLight}
            </span>
          </button>
        </div>

        <div className="settings-group">
          <div className="settings-group-label">{copy.settingsLanguage}</div>
          <div className="settings-lang-row">
            <button
              className={`settings-lang-btn${locale === "ru" ? " is-active" : ""}`}
              onClick={() => onSetLocale("ru")}
              type="button"
            >
              Русский
            </button>
            <button
              className={`settings-lang-btn${locale === "en" ? " is-active" : ""}`}
              onClick={() => onSetLocale("en")}
              type="button"
            >
              English
            </button>
          </div>
        </div>
      </div>
    </>
  );
}

/* ── Device verification ──────────────────────────────── */

function SettingsDevices({
  copy,
  devices,
  selectedDeviceId,
  trustedCount,
  pendingCount,
  busy,
  onBack,
  onSelect,
  onVerifyQr,
  onVerifySafety,
}: {
  copy: CopyBundle;
  devices: VerificationDeviceView[];
  selectedDeviceId: string | null;
  trustedCount: number;
  pendingCount: number;
  busy: boolean;
  onBack: () => void;
  onSelect: (id: string) => void;
  onVerifyQr: () => void;
  onVerifySafety: () => void;
}) {
  const selected =
    devices.find((d) => d.deviceId === selectedDeviceId) ?? null;

  return (
    <>
      <div className="settings-header">
        <button className="settings-back-btn" onClick={onBack} type="button">
          ‹
        </button>
        <h2 className="settings-title">{copy.settingsDevices}</h2>
      </div>
      <div className="settings-body">
        <div className="settings-stats-row">
          <div className="settings-stat">
            <span className="settings-stat-value">{trustedCount}</span>
            <span className="settings-stat-label">{copy.trustedDevices}</span>
          </div>
          <div className="settings-stat">
            <span className="settings-stat-value">{pendingCount}</span>
            <span className="settings-stat-label">{copy.pendingDevices}</span>
          </div>
        </div>

        <div className="settings-device-list">
          {devices.map((device) => (
            <button
              key={device.deviceId}
              className={`settings-device-item${device.deviceId === selectedDeviceId ? " is-active" : ""}`}
              onClick={() => onSelect(device.deviceId)}
              type="button"
            >
              <div className="settings-device-info">
                <strong>{device.deviceName}</strong>
                <span>{device.memberName}</span>
              </div>
              <span
                className={`settings-device-badge ${device.state}`}
              >
                {copy.verificationStates[device.state]}
              </span>
            </button>
          ))}
        </div>

        {selected ? (
          <div className="settings-device-detail">
            <div className="settings-group">
              <div className="settings-group-label">
                {selected.deviceName} — {selected.memberName}
              </div>
              <div className="settings-info-row">
                <span className="settings-info-label">{copy.safetyNumber}</span>
                <code className="settings-info-code">
                  {selected.safetyNumber}
                </code>
              </div>
              <div className="settings-info-row">
                <span className="settings-info-label">{copy.qrPayload}</span>
                <code className="settings-info-code settings-info-hex">
                  {selected.qrPayloadHex}
                </code>
              </div>
            </div>
            <div className="settings-actions">
              <button
                className="settings-btn secondary"
                disabled={busy}
                onClick={onVerifyQr}
                type="button"
              >
                {copy.scanQr}
              </button>
              <button
                className="settings-btn primary"
                disabled={busy}
                onClick={onVerifySafety}
                type="button"
              >
                {copy.safetyCheck}
              </button>
            </div>
          </div>
        ) : null}
      </div>
    </>
  );
}

/* ── Network / peers ──────────────────────────────────── */

function SettingsNetwork({
  copy,
  transport,
  peers,
  peerStates,
  busy,
  onBack,
  onRefresh,
  onStartChat,
}: {
  copy: CopyBundle;
  transport: ClientSnapshot["transportStatus"] | null;
  peers: ClientSnapshot["peers"];
  peerStates: Record<string, string>;
  busy: boolean;
  onBack: () => void;
  onRefresh: () => void;
  onStartChat: (deviceId: string) => void;
}) {
  return (
    <>
      <div className="settings-header">
        <button className="settings-back-btn" onClick={onBack} type="button">
          ‹
        </button>
        <h2 className="settings-title">{copy.settingsNetwork}</h2>
      </div>
      <div className="settings-body">
        {transport ? (
          <div className="settings-group">
            <div className="settings-group-label">{copy.discoveryTransport}</div>
            <div className="settings-info-row">
              <span className="settings-info-label">{copy.discoveryLabel}</span>
              <span className="settings-info-value">
                {transport.discoveryMode}
              </span>
            </div>
            <div className="settings-info-row">
              <span className="settings-info-label">{copy.transportLabel}</span>
              <span className="settings-info-value">
                {transport.transportMode}
              </span>
            </div>
            <div className="settings-info-row">
              <span className="settings-info-label">{copy.cryptoLabel}</span>
              <span className="settings-info-value">
                {transport.cryptoMode}
              </span>
            </div>
            <div className="settings-info-row">
              <span className="settings-info-label">{copy.storageLabel}</span>
              <span className="settings-info-value">
                {transport.storageMode}
              </span>
            </div>
            <div className="settings-info-row">
              <span className="settings-info-label">Route</span>
              <span className="settings-info-value">
                {transport.activeRoute}
              </span>
            </div>
          </div>
        ) : null}

        <div className="settings-group">
          <div className="settings-group-label">
            {copy.peers} ({peers.length})
          </div>
          {peers.map((peer) => (
            <div className="settings-peer-row" key={peer.deviceId}>
              <div className="settings-peer-info">
                <strong>{peer.deviceName}</strong>
                <span className="settings-peer-endpoint">
                  {peer.endpoint}
                </span>
              </div>
              <div className="settings-peer-actions">
                <span className={`settings-peer-state ${peer.state}`}>
                  {peerStates[peer.state] ?? peer.state}
                </span>
                <button
                  className="settings-peer-chat-btn"
                  disabled={busy}
                  onClick={() => onStartChat(peer.deviceId)}
                  title={`Start chat with ${peer.deviceName}`}
                  type="button"
                >
                  ✉
                </button>
              </div>
            </div>
          ))}
        </div>

        <div className="settings-actions">
          <button
            className="settings-btn secondary"
            disabled={busy}
            onClick={onRefresh}
            type="button"
          >
            {copy.refreshPeers}
          </button>
        </div>
      </div>
    </>
  );
}

/* ── Relay / invite ───────────────────────────────────── */

function SettingsRelay({
  copy,
  inviteDraft,
  contactInviteDraft,
  generatedContactInvite,
  busy,
  onBack,
  onInviteDraftChange,
  onPreview,
  onAccept,
  onContactInviteDraftChange,
  onCreateContactInvite,
  onPreviewContactInvite,
  onAcceptContactInvite,
}: {
  copy: CopyBundle;
  inviteDraft: string;
  contactInviteDraft: string;
  generatedContactInvite: string;
  busy: boolean;
  onBack: () => void;
  onInviteDraftChange: (v: string) => void;
  onPreview: () => void;
  onAccept: () => void;
  onContactInviteDraftChange: (v: string) => void;
  onCreateContactInvite: () => void;
  onPreviewContactInvite: () => void;
  onAcceptContactInvite: () => void;
}) {
  return (
    <>
      <div className="settings-header">
        <button className="settings-back-btn" onClick={onBack} type="button">
          ‹
        </button>
        <h2 className="settings-title">{copy.settingsRelay}</h2>
      </div>
      <div className="settings-body">
        <div className="settings-group">
          <div className="settings-group-label">{copy.settingsJoinRelay}</div>
          <textarea
            className="settings-textarea"
            onChange={(e) => onInviteDraftChange(e.target.value)}
            placeholder="localmessenger://join..."
            rows={3}
            value={inviteDraft}
          />
          <div className="settings-actions">
            <button
              className="settings-btn secondary"
              disabled={busy || inviteDraft.trim().length === 0}
              onClick={onPreview}
              type="button"
            >
              {copy.settingsPreviewInvite}
            </button>
            <button
              className="settings-btn primary"
              disabled={busy || inviteDraft.trim().length === 0}
              onClick={onAccept}
              type="button"
            >
              {copy.settingsJoinButton}
            </button>
          </div>
        </div>

        <div className="settings-group">
          <div className="settings-group-label">Create contact invite</div>
          <div className="settings-actions">
            <button
              className="settings-btn primary"
              disabled={busy}
              onClick={onCreateContactInvite}
              type="button"
            >
              Generate contact invite
            </button>
          </div>
          {generatedContactInvite ? (
            <textarea
              className="settings-textarea"
              readOnly
              rows={4}
              value={generatedContactInvite}
            />
          ) : null}
        </div>

        <div className="settings-group">
          <div className="settings-group-label">Accept contact invite</div>
          <textarea
            className="settings-textarea"
            onChange={(e) => onContactInviteDraftChange(e.target.value)}
            placeholder="localmessenger://contact..."
            rows={4}
            value={contactInviteDraft}
          />
          <div className="settings-actions">
            <button
              className="settings-btn secondary"
              disabled={busy || contactInviteDraft.trim().length === 0}
              onClick={onPreviewContactInvite}
              type="button"
            >
              Preview contact
            </button>
            <button
              className="settings-btn primary"
              disabled={busy || contactInviteDraft.trim().length === 0}
              onClick={onAcceptContactInvite}
              type="button"
            >
              Add contact
            </button>
          </div>
        </div>
      </div>
    </>
  );
}

/* ── Updates ──────────────────────────────────────────── */

function SettingsUpdates({
  copy,
  updater,
  busy,
  onBack,
  onCheck,
}: {
  copy: CopyBundle;
  updater: ClientSnapshot["updater"] | null;
  busy: boolean;
  onBack: () => void;
  onCheck: () => void;
}) {
  return (
    <>
      <div className="settings-header">
        <button className="settings-back-btn" onClick={onBack} type="button">
          ‹
        </button>
        <h2 className="settings-title">{copy.settingsUpdates}</h2>
      </div>
      <div className="settings-body">
        {updater ? (
          <div className="settings-group">
            <div className="settings-info-row">
              <span className="settings-info-label">{copy.updaterVersion}</span>
              <span className="settings-info-value">
                v{updater.currentVersion}
              </span>
            </div>
            <div className="settings-info-row">
              <span className="settings-info-label">{copy.updaterChannel}</span>
              <span className="settings-info-value">{updater.channel}</span>
            </div>
            <div className="settings-info-row">
              <span className="settings-info-label">
                {copy.settingsLastCheck}
              </span>
              <span className="settings-info-value">
                {updater.lastCheckedLabel}
              </span>
            </div>
            <div className="settings-info-row">
              <span className="settings-info-label">Auto-update</span>
              <span className="settings-info-value">
                {updater.canAutoUpdate ? "enabled" : "manual"}
              </span>
            </div>
          </div>
        ) : null}

        <div className="settings-group">
          <div className="settings-info-text">
            {updater?.statusLabel ?? "..."}
          </div>
        </div>

        <div className="settings-actions">
          <button
            className="settings-btn primary"
            disabled={busy}
            onClick={onCheck}
            type="button"
          >
            {copy.updaterCheck}
          </button>
        </div>
      </div>
    </>
  );
}
