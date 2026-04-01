import type { LocalProfileView, OnboardingView } from "../types";

type OnboardingPanelProps = {
  onboarding: OnboardingView;
  localProfile: LocalProfileView;
  inviteDraft: string;
  busy: boolean;
  onInviteDraftChange: (value: string) => void;
  onPreview: () => void;
  onAccept: () => void;
};

type StepState = "complete" | "active" | "upcoming";

interface StepDef {
  number: number;
  title: string;
  state: StepState;
  content?: React.ReactNode;
}

function stepCircleStyle(state: StepState): React.CSSProperties {
  const base: React.CSSProperties = {
    flexShrink: 0,
    width: 28,
    height: 28,
    borderRadius: "50%",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    fontSize: 12,
    fontWeight: 700,
    transition: "background 0.2s, border-color 0.2s",
  };
  if (state === "complete") {
    return {
      ...base,
      background: "#2b5278",
      border: "2px solid #5ea8de",
      color: "#5ea8de",
    };
  }
  if (state === "active") {
    return {
      ...base,
      background: "#5ea8de",
      border: "2px solid #5ea8de",
      color: "#fff",
    };
  }
  return {
    ...base,
    background: "transparent",
    border: "2px solid #3a4a5a",
    color: "#5a7080",
  };
}

function stepConnectorStyle(state: StepState): React.CSSProperties {
  return {
    width: 2,
    height: 24,
    margin: "3px auto",
    background: state === "complete" ? "#2b5278" : "#1e2d3d",
    borderRadius: 1,
  };
}

function stepTitleStyle(state: StepState): React.CSSProperties {
  if (state === "active") {
    return { fontWeight: 600, color: "#e8f4ff", fontSize: 14 };
  }
  if (state === "complete") {
    return {
      fontWeight: 500,
      color: "#7a9ab5",
      fontSize: 14,
      textDecoration: "line-through",
      textDecorationColor: "#3a5a78",
    };
  }
  return { fontWeight: 400, color: "#4a6070", fontSize: 14 };
}

export function OnboardingPanel({
  onboarding,
  localProfile,
  inviteDraft,
  busy,
  onInviteDraftChange,
  onPreview,
  onAccept,
}: OnboardingPanelProps) {
  const hasDeviceId =
    localProfile.activeDeviceId != null &&
    localProfile.activeDeviceId.trim().length > 0;

  // Step 3 is considered done if the status label no longer reads as "not configured"
  const statusLower = onboarding.statusLabel.toLowerCase();
  const hasJoinedRelay =
    hasDeviceId &&
    !statusLower.includes("not configured") &&
    !statusLower.includes("pending") &&
    !statusLower.includes("waiting") &&
    statusLower.length > 0 &&
    (statusLower.includes("connect") ||
      statusLower.includes("online") ||
      statusLower.includes("ready") ||
      statusLower.includes("joined") ||
      statusLower.includes("active"));

  const hasVerifiedContacts = localProfile.trustedDeviceCount > 0;

  // Compute per-step states
  const step1State: StepState = "complete";
  const step2State: StepState = hasDeviceId ? "complete" : "active";
  const step3State: StepState = hasJoinedRelay
    ? "complete"
    : hasDeviceId
      ? "active"
      : "upcoming";
  const step4State: StepState = hasVerifiedContacts
    ? "complete"
    : hasJoinedRelay
      ? "active"
      : "upcoming";

  const shortDeviceId = hasDeviceId
    ? localProfile.activeDeviceId.length > 16
      ? `${localProfile.activeDeviceId.slice(0, 8)}…${localProfile.activeDeviceId.slice(-8)}`
      : localProfile.activeDeviceId
    : null;

  const steps: StepDef[] = [
    {
      number: 1,
      title: "Download and install",
      state: step1State,
    },
    {
      number: 2,
      title: "Device identity generated",
      state: step2State,
      content: hasDeviceId ? (
        <div
          style={{
            marginTop: 8,
            padding: "6px 10px",
            borderRadius: 6,
            background: "rgba(43,82,120,0.35)",
            border: "1px solid #2b5278",
          }}
        >
          <span
            style={{
              fontSize: 11,
              color: "#7a9ab5",
              display: "block",
              marginBottom: 2,
            }}
          >
            Device ID
          </span>
          <code
            style={{
              fontSize: 11,
              color: "#a8d0f0",
              fontFamily: '"Roboto Mono", "Fira Mono", monospace',
              letterSpacing: "0.03em",
              wordBreak: "break-all",
            }}
          >
            {shortDeviceId}
          </code>
          {localProfile.activeDeviceName ? (
            <span
              style={{
                display: "block",
                marginTop: 4,
                fontSize: 11,
                color: "#7a9ab5",
              }}
            >
              {localProfile.activeDeviceName}
            </span>
          ) : null}
        </div>
      ) : (
        <p style={{ marginTop: 6, fontSize: 12, color: "#4a6070" }}>
          Your device identity will be created automatically on first run.
        </p>
      ),
    },
    {
      number: 3,
      title: "Join a relay",
      state: step3State,
      content:
        step3State === "active" || step3State === "complete" ? (
          <div style={{ marginTop: 10 }}>
            <p style={{ fontSize: 12, color: "#7a9ab5", marginBottom: 8 }}>
              {onboarding.statusLabel}
            </p>

            <textarea
              className="composer-input"
              onChange={(event) => onInviteDraftChange(event.target.value)}
              placeholder="Paste localmessenger://join invite link..."
              rows={3}
              value={inviteDraft}
              style={{ width: "100%", marginBottom: 8 }}
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
              <div className="verification-detail" style={{ marginTop: 10 }}>
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
          </div>
        ) : null,
    },
    {
      number: 4,
      title: "Verify contacts",
      state: step4State,
      content:
        step4State === "active" ? (
          <p style={{ marginTop: 6, fontSize: 12, color: "#7a9ab5" }}>
            Open{" "}
            <strong style={{ color: "#a8d0f0" }}>Settings → Devices</strong> to
            verify your contacts using a safety number or QR code.
          </p>
        ) : step4State === "complete" ? (
          <p style={{ marginTop: 6, fontSize: 12, color: "#5a7a60" }}>
            {localProfile.trustedDeviceCount} device
            {localProfile.trustedDeviceCount === 1 ? "" : "s"} verified.
          </p>
        ) : null,
    },
  ];

  return (
    <section className="panel context-card">
      <div className="panel-header">
        <div>
          <p className="eyebrow">Onboarding</p>
          <h3>Getting started</h3>
        </div>
      </div>

      <ol
        style={{
          listStyle: "none",
          padding: 0,
          margin: 0,
        }}
      >
        {steps.map((step, index) => {
          const isLast = index === steps.length - 1;
          return (
            <li
              key={step.number}
              style={{ display: "flex", gap: 12, alignItems: "flex-start" }}
            >
              {/* Left column: circle + connector */}
              <div
                style={{
                  display: "flex",
                  flexDirection: "column",
                  alignItems: "center",
                  minWidth: 28,
                }}
              >
                <div style={stepCircleStyle(step.state)}>
                  {step.state === "complete" ? (
                    // Checkmark
                    <svg
                      width="12"
                      height="12"
                      viewBox="0 0 12 12"
                      fill="none"
                      xmlns="http://www.w3.org/2000/svg"
                    >
                      <path
                        d="M2 6L5 9L10 3"
                        stroke="currentColor"
                        strokeWidth="2"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                      />
                    </svg>
                  ) : (
                    step.number
                  )}
                </div>
                {!isLast && <div style={stepConnectorStyle(step.state)} />}
              </div>

              {/* Right column: title + content */}
              <div
                style={{
                  flex: 1,
                  paddingBottom: isLast ? 0 : 16,
                  paddingTop: 3,
                }}
              >
                <span style={stepTitleStyle(step.state)}>{step.title}</span>
                {step.content ?? null}
              </div>
            </li>
          );
        })}
      </ol>
    </section>
  );
}
