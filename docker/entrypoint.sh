#!/bin/sh
set -e

DATA_DIR="${DATA_DIR:-/data}"
DB_PATH="${DB_PATH:-${DATA_DIR}/relay.db}"
CERT_PATH="${CERT_PATH:-${DATA_DIR}/relay-cert.der}"
KEY_PATH="${KEY_PATH:-${DATA_DIR}/relay-key.der}"
SERVER_NAME="${SERVER_NAME:-relay.local}"
INVITE_SECRET="${INVITE_SECRET:-changeme-replace-in-production}"
BIND_ADDR="${BIND_ADDR:-0.0.0.0:7443}"

mkdir -p "${DATA_DIR}"

# Generate self-signed certificate if not already present.
if [ ! -f "${CERT_PATH}" ] || [ ! -f "${KEY_PATH}" ]; then
    echo "[entrypoint] Generating self-signed certificate for '${SERVER_NAME}'..."
    localmessenger_server gen-cert \
        --server-name "${SERVER_NAME}" \
        --cert "${CERT_PATH}" \
        --key  "${KEY_PATH}"
fi

echo "[entrypoint] Starting relay server on ${BIND_ADDR} (${SERVER_NAME})"
exec localmessenger_server serve \
    --bind              "${BIND_ADDR}" \
    --server-name       "${SERVER_NAME}" \
    --cert              "${CERT_PATH}" \
    --key               "${KEY_PATH}" \
    --db                "${DB_PATH}" \
    --invite-secret     "${INVITE_SECRET}" \
    ${RATE_WINDOW_SECONDS:+--rate-window-seconds      "${RATE_WINDOW_SECONDS}"} \
    ${PEER_FRAME_LIMIT:+--peer-frame-limit             "${PEER_FRAME_LIMIT}"} \
    ${BLOB_REQUEST_LIMIT:+--blob-request-limit         "${BLOB_REQUEST_LIMIT}"} \
    ${BLOB_CHUNK_BYTE_LIMIT:+--blob-chunk-byte-limit   "${BLOB_CHUNK_BYTE_LIMIT}"} \
    ${HEALTH_CHECK_LIMIT:+--health-check-limit         "${HEALTH_CHECK_LIMIT}"}
