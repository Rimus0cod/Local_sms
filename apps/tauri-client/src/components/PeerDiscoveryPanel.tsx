import type { CopyBundle } from "../i18n";
import type { PeerState, PeerView, TransportStatusView } from "../types";

type PeerDiscoveryPanelProps = {
  copy: CopyBundle;
  refreshLabel: string;
  peers: PeerView[];
  transportStatus: TransportStatusView;
  peerStates: Record<PeerState, string>;
  busy: boolean;
  onRefresh: () => void;
};

export function PeerDiscoveryPanel({
  copy,
  refreshLabel,
  peers,
  transportStatus,
  peerStates,
  busy,
  onRefresh,
}: PeerDiscoveryPanelProps) {
  return (
    <section className="panel context-card">
      <div className="panel-header">
        <div>
          <p className="eyebrow">{copy.peers}</p>
          <h3>{copy.discoveryTransport}</h3>
        </div>
        <button
          className="secondary-button"
          disabled={busy}
          onClick={onRefresh}
          type="button"
        >
          {refreshLabel}
        </button>
      </div>

      <div className="status-grid">
        <div className="status-tile">
          <span>{copy.discoveryLabel}</span>
          <strong>{transportStatus.discoveryMode}</strong>
        </div>
        <div className="status-tile">
          <span>{copy.transportLabel}</span>
          <strong>{transportStatus.transportMode}</strong>
        </div>
        <div className="status-tile">
          <span>{copy.cryptoLabel}</span>
          <strong>{transportStatus.cryptoMode}</strong>
        </div>
        <div className="status-tile">
          <span>{copy.storageLabel}</span>
          <strong>{transportStatus.storageMode}</strong>
        </div>
      </div>

      <div className="peer-list">
        {peers.map((peer) => (
          <article className="peer-card" key={peer.deviceId}>
            <div className="peer-card-topline">
              <div>
                <strong>{peer.deviceName}</strong>
                <p>{peer.memberId}</p>
              </div>
              <span className={`peer-state peer-${peer.state}`}>
                {peerStates[peer.state]}
              </span>
            </div>
            <p className="peer-endpoint">{peer.endpoint}</p>
            <p className="peer-hostname">{peer.hostname ?? "no hostname"}</p>
            <div className="chat-meta-row">
              {peer.capabilities.map((capability) => (
                <span className="meta-pill" key={capability}>
                  {capability}
                </span>
              ))}
            </div>
            <div className="chat-meta-row">
              <span>
                {peer.trustLabel === "verified"
                  ? copy.verificationStates.verified
                  : copy.verificationStates.pending}
              </span>
              <span>{peer.lastSeenLabel}</span>
            </div>
          </article>
        ))}
      </div>
    </section>
  );
}
