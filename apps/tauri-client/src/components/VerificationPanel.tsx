import type { CopyBundle } from "../i18n";
import type { VerificationDeviceView } from "../types";

type VerificationPanelProps = {
  copy: CopyBundle;
  devices: VerificationDeviceView[];
  selectedDeviceId: string | null;
  trustedDeviceCount: number;
  pendingDeviceCount: number;
  busy: boolean;
  onSelect: (deviceId: string) => void;
  onVerifyQr: () => void;
  onVerifySafety: () => void;
};

export function VerificationPanel({
  copy,
  devices,
  selectedDeviceId,
  trustedDeviceCount,
  pendingDeviceCount,
  busy,
  onSelect,
  onVerifyQr,
  onVerifySafety,
}: VerificationPanelProps) {
  const selectedDevice =
    devices.find((device) => device.deviceId === selectedDeviceId) ?? null;

  return (
    <section className="panel context-card">
      <div className="panel-header">
        <div>
          <p className="eyebrow">{copy.verification}</p>
          <h3>{copy.trustWorkspace}</h3>
        </div>
        <div className="chat-meta-row">
          <span className="meta-pill">
            {copy.trustedDevices}: {trustedDeviceCount}
          </span>
          <span className="meta-pill">
            {copy.pendingDevices}: {pendingDeviceCount}
          </span>
        </div>
      </div>

      <div className="verification-layout">
        <div className="verification-device-list">
          {devices.map((device) => (
            <button
              key={device.deviceId}
              className={`verification-device${device.deviceId === selectedDeviceId ? " is-active" : ""}`}
              onClick={() => onSelect(device.deviceId)}
              type="button"
            >
              <strong>{device.deviceName}</strong>
              <span>{device.memberName}</span>
              <span>
                {copy.verificationStates[device.state]}
                {device.method ? ` • ${copy.verificationMethods[device.method]}` : ""}
              </span>
            </button>
          ))}
        </div>

        <div className="verification-detail">
          {selectedDevice ? (
            <>
              <div className="verification-summary-card">
                <p className="eyebrow">{selectedDevice.memberName}</p>
                <h4>{selectedDevice.deviceName}</h4>
                <p>
                  {copy.verificationStates[selectedDevice.state]}
                  {selectedDevice.method
                    ? ` • ${copy.verifiedVia}: ${copy.verificationMethods[selectedDevice.method]}`
                    : ""}
                </p>
              </div>
              <div className="verification-detail-block">
                <span>{copy.safetyNumber}</span>
                <code>{selectedDevice.safetyNumber}</code>
              </div>
              <div className="verification-detail-block">
                <span>{copy.qrPayload}</span>
                <code className="qr-payload">{selectedDevice.qrPayloadHex}</code>
              </div>
              <div className="verification-actions">
                <button
                  className="secondary-button"
                  disabled={busy}
                  onClick={onVerifyQr}
                  type="button"
                >
                  {copy.scanQr}
                </button>
                <button
                  className="primary-button"
                  disabled={busy}
                  onClick={onVerifySafety}
                  type="button"
                >
                  {copy.safetyCheck}
                </button>
              </div>
            </>
          ) : (
            <p className="empty-copy">{copy.noVerificationDevice}</p>
          )}
        </div>
      </div>
    </section>
  );
}
