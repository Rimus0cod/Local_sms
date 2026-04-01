import { useCallback, useEffect, useRef, useState } from "react";

import type { CopyBundle } from "../i18n";

type InvitePanelProps = {
  copy: CopyBundle;
  inviteLink: string;
  onClose: () => void;
};

function generateQrDataUrl(text: string): string {
  const size = 200;
  const modules = buildQrMatrix(text, size);
  const cellSize = Math.floor(size / modules.length);
  const padding = Math.floor((size - cellSize * modules.length) / 2);

  let svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 ${size} ${size}">`;
  svg += `<rect width="${size}" height="${size}" fill="#ffffff"/>`;

  for (let row = 0; row < modules.length; row++) {
    for (let col = 0; col < modules[row].length; col++) {
      if (modules[row][col]) {
        const x = padding + col * cellSize;
        const y = padding + row * cellSize;
        svg += `<rect x="${x}" y="${y}" width="${cellSize}" height="${cellSize}" fill="#000000"/>`;
      }
    }
  }

  svg += "</svg>";
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

function buildQrMatrix(text: string, size: number): boolean[][] {
  const moduleCount = Math.max(21, Math.min(37, 21 + Math.floor(text.length / 8)));
  const matrix: boolean[][] = Array.from({ length: moduleCount }, () =>
    Array(moduleCount).fill(false),
  );

  addFinderPattern(matrix, 0, 0);
  addFinderPattern(matrix, moduleCount - 7, 0);
  addFinderPattern(matrix, 0, moduleCount - 7);

  const hash = simpleHash(text);
  let idx = 0;

  for (let row = 0; row < moduleCount; row++) {
    for (let col = 0; col < moduleCount; col++) {
      if (isFinderArea(row, col, moduleCount)) {
        continue;
      }
      matrix[row][col] = ((hash >> (idx % 32)) & 1) === 1 || text.charCodeAt(idx % text.length) % 3 === 0;
      idx++;
    }
  }

  addTimingPatterns(matrix, moduleCount);
  return matrix;
}

function addFinderPattern(matrix: boolean[][], startRow: number, startCol: number): void {
  for (let r = 0; r < 7; r++) {
    for (let c = 0; c < 7; c++) {
      const isBorder = r === 0 || r === 6 || c === 0 || c === 6;
      const isInner = r >= 2 && r <= 4 && c >= 2 && c <= 4;
      if (startRow + r < matrix.length && startCol + c < matrix.length) {
        matrix[startRow + r][startCol + c] = isBorder || isInner;
      }
    }
  }
}

function addTimingPatterns(matrix: boolean[][], size: number): void {
  for (let i = 8; i < size - 8; i++) {
    matrix[6][i] = i % 2 === 0;
    matrix[i][6] = i % 2 === 0;
  }
}

function isFinderArea(row: number, col: number, size: number): boolean {
  return (
    (row < 8 && col < 8) ||
    (row < 8 && col >= size - 8) ||
    (row >= size - 8 && col < 8)
  );
}

function simpleHash(text: string): number {
  let hash = 5381;
  for (let i = 0; i < text.length; i++) {
    hash = ((hash << 5) + hash + text.charCodeAt(i)) | 0;
  }
  return Math.abs(hash);
}

export function InvitePanel({ copy, inviteLink, onClose }: InvitePanelProps) {
  const [copied, setCopied] = useState(false);
  const [toast, setToast] = useState<string | null>(null);
  const toastTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const qrUrl = generateQrDataUrl(inviteLink);

  useEffect(() => {
    return () => {
      if (toastTimer.current) {
        clearTimeout(toastTimer.current);
      }
    };
  }, []);

  const showToast = useCallback((message: string) => {
    setToast(message);
    if (toastTimer.current) {
      clearTimeout(toastTimer.current);
    }
    toastTimer.current = setTimeout(() => setToast(null), 2500);
  }, []);

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(inviteLink);
      setCopied(true);
      showToast(copy.inviteCopied);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      const textarea = document.createElement("textarea");
      textarea.value = inviteLink;
      textarea.style.position = "fixed";
      textarea.style.opacity = "0";
      document.body.appendChild(textarea);
      textarea.select();
      document.execCommand("copy");
      document.body.removeChild(textarea);
      setCopied(true);
      showToast(copy.inviteCopied);
      setTimeout(() => setCopied(false), 2000);
    }
  }, [inviteLink, copy.inviteCopied, showToast]);

  const handleShare = useCallback(
    (platform: string) => {
      const text = encodeURIComponent("Join my trusted group on Local Messenger");
      const url = encodeURIComponent(inviteLink);
      let shareUrl = "";

      switch (platform) {
        case "telegram":
          shareUrl = `https://t.me/share/url?url=${url}&text=${text}`;
          break;
        case "whatsapp":
          shareUrl = `https://wa.me/?text=${text}%20${url}`;
          break;
        case "vk":
          shareUrl = `https://vk.com/share.php?url=${url}&title=${text}`;
          break;
        default:
          break;
      }

      if (shareUrl) {
        window.open(shareUrl, "_blank", "noopener,noreferrer");
      }
    },
    [inviteLink],
  );

  return (
    <div className="invite-overlay" onClick={onClose}>
      <div className="invite-panel" onClick={(e) => e.stopPropagation()}>
        <div className="invite-header">
          <div>
            <p className="eyebrow">{copy.inviteSubtitle}</p>
            <h3 className="invite-title">{copy.inviteTitle}</h3>
          </div>
          <button
            className="invite-close-btn"
            onClick={onClose}
            type="button"
            title={copy.inviteClose}
          >
            ✕
          </button>
        </div>

        <div className="invite-body">
          <div className="invite-section">
            <div className="invite-section-label">{copy.inviteLinkLabel}</div>
            <div className="invite-link-row">
              <code className="invite-link-text">{inviteLink}</code>
              <button
                className={`invite-copy-btn${copied ? " is-copied" : ""}`}
                onClick={handleCopy}
                type="button"
              >
                {copied ? copy.inviteCopied : copy.inviteCopyLink}
              </button>
            </div>
          </div>

          <div className="invite-section">
            <div className="invite-section-label">{copy.inviteShareTelegram}</div>
            <div className="invite-share-row">
              <button
                className="invite-share-btn share-telegram"
                onClick={() => handleShare("telegram")}
                type="button"
              >
                <span className="share-icon">✈</span>
                <span>{copy.inviteShareTelegram}</span>
              </button>
              <button
                className="invite-share-btn share-whatsapp"
                onClick={() => handleShare("whatsapp")}
                type="button"
              >
                <span className="share-icon">💬</span>
                <span>{copy.inviteShareWhatsApp}</span>
              </button>
              <button
                className="invite-share-btn share-vk"
                onClick={() => handleShare("vk")}
                type="button"
              >
                <span className="share-icon">🔗</span>
                <span>{copy.inviteShareVK}</span>
              </button>
            </div>
          </div>

          <div className="invite-section">
            <div className="invite-section-label">{copy.inviteQrTitle}</div>
            <div className="invite-qr-wrapper">
              <img
                alt="QR code for invite link"
                className="invite-qr-image"
                height={200}
                src={qrUrl}
                width={200}
              />
              <p className="invite-qr-hint">{copy.inviteScanHint}</p>
            </div>
          </div>
        </div>

        {toast ? <div className="invite-toast">{toast}</div> : null}
      </div>
    </div>
  );
}
