import type { OnboardingView } from "../types";

type OnboardingPanelProps = {
  onboarding: OnboardingView;
  inviteDraft: string;
  busy: boolean;
  onInviteDraftChange: (value: string) => void;
  onPreview: () => void;
  onAccept: () => void;
};

export function OnboardingPanel({
  onboarding,
  inviteDraft,
  busy,
  onInviteDraftChange,
  onPreview,
  onAccept,
}: OnboardingPanelProps) {
  return (
    <section className="panel context-card">
      <div className="panel-header">
        <div>
          <p className="eyebrow">Onboarding</p>
          <h3>Invite join flow</h3>
        </div>
      </div>

      <p className="chat-presence">{onboarding.statusLabel}</p>

      <textarea
        className="composer-input"
        onChange={(event) => onInviteDraftChange(event.target.value)}
        placeholder="Paste localmessenger://join invite link..."
        rows={3}
        value={inviteDraft}
      />

      <div className="verification-actions">
        <button
          className="secondary-button"
          disabled={busy || inviteDraft.trim().length === 0}
          onClick={onPreview}
          type="button"
        >
          Preview invite
        </button>
        <button
          className="primary-button"
          disabled={busy || inviteDraft.trim().length === 0}
          onClick={onAccept}
          type="button"
        >
          Join relay
        </button>
      </div>

      {onboarding.invitePreview ? (
        <div className="verification-detail">
          <div className="verification-detail-block">
            <span>Label</span>
            <strong>{onboarding.invitePreview.label}</strong>
          </div>
          <div className="verification-detail-block">
            <span>Server</span>
            <strong>{onboarding.invitePreview.serverAddr}</strong>
          </div>
          <div className="verification-detail-block">
            <span>Name</span>
            <strong>{onboarding.invitePreview.serverName}</strong>
          </div>
          <div className="verification-detail-block">
            <span>Expiry</span>
            <strong>{onboarding.invitePreview.expiresAtLabel}</strong>
          </div>
          <div className="verification-detail-block">
            <span>Max uses</span>
            <strong>{onboarding.invitePreview.maxUses}</strong>
          </div>
        </div>
      ) : null}
    </section>
  );
}
