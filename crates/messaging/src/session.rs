use localmessenger_core::Device;
use localmessenger_crypto::{
    DoubleRatchet, EncryptedMessage, IdentityKeyPair, LocalPrekeyStore, PublicPrekeyBundle,
    RatchetStateSnapshot, SessionRole, accept_session, initiate_session,
};
use localmessenger_transport::{TransportConnection, TransportFrame};
use rand_core::OsRng;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::error::MessagingError;
use crate::handshake::{
    SECURE_SESSION_VERSION, SessionBinding, SessionPeer, SessionRequest, SessionResponse,
};

pub fn transport_certificate_sha256(certificate_der: &[u8]) -> [u8; 32] {
    Sha256::digest(certificate_der).into()
}

#[derive(Debug, Clone)]
pub struct RemoteSessionOffer {
    remote_device: Device,
    prekey_bundle: PublicPrekeyBundle,
    transport_certificate_sha256: [u8; 32],
}

impl RemoteSessionOffer {
    pub fn from_parts(
        remote_device: Device,
        prekey_bundle: PublicPrekeyBundle,
        transport_certificate_sha256: [u8; 32],
    ) -> Result<Self, MessagingError> {
        prekey_bundle.verify()?;
        if prekey_bundle.identity != *remote_device.identity_keys() {
            return Err(MessagingError::RemoteOfferMismatch(
                "prekey bundle identity does not match remote device identity",
            ));
        }

        Ok(Self {
            remote_device,
            prekey_bundle,
            transport_certificate_sha256,
        })
    }

    pub fn remote_device(&self) -> &Device {
        &self.remote_device
    }

    pub fn prekey_bundle(&self) -> &PublicPrekeyBundle {
        &self.prekey_bundle
    }

    pub fn transport_certificate_sha256(&self) -> [u8; 32] {
        self.transport_certificate_sha256
    }
}

pub struct SessionInitiator {
    local_device: Device,
    local_identity: IdentityKeyPair,
}

impl SessionInitiator {
    pub fn new(
        local_device: Device,
        local_identity: IdentityKeyPair,
    ) -> Result<Self, MessagingError> {
        validate_local_device_identity(&local_device, &local_identity)?;
        Ok(Self {
            local_device,
            local_identity,
        })
    }

    pub async fn establish(
        &self,
        connection: TransportConnection,
        remote_offer: &RemoteSessionOffer,
        trusted_remote_transport_certificate_der: &[u8],
    ) -> Result<SecureSession, MessagingError> {
        let trusted_remote_transport_sha256 =
            transport_certificate_sha256(trusted_remote_transport_certificate_der);
        if trusted_remote_transport_sha256 != remote_offer.transport_certificate_sha256() {
            return Err(MessagingError::TransportBindingMismatch);
        }

        let mut rng = OsRng;
        let bootstrap =
            initiate_session(&mut rng, &self.local_identity, remote_offer.prekey_bundle())?;
        let initiator_peer = SessionPeer::from_device(&self.local_device);
        let expected_responder = SessionPeer::from_device(remote_offer.remote_device());

        let request = SessionRequest {
            version: SECURE_SESSION_VERSION,
            initiator: initiator_peer.clone(),
            expected_responder: expected_responder.clone(),
            expected_responder_transport_certificate_sha256: trusted_remote_transport_sha256,
            x3dh: bootstrap.handshake,
        };

        connection
            .send_frame(&TransportFrame::Handshake(bincode::serialize(&request)?))
            .await?;

        let frame = connection.receive_frame().await?;
        let response_bytes = expect_handshake(frame, "expected handshake response")?;
        let response: SessionResponse = bincode::deserialize(&response_bytes)?;
        ensure_supported_version(response.version)?;

        if response.responder != expected_responder {
            return Err(MessagingError::RemoteBindingMismatch(
                "responder device metadata",
            ));
        }
        if response.responder_transport_certificate_sha256 != trusted_remote_transport_sha256 {
            return Err(MessagingError::TransportBindingMismatch);
        }

        let binding = SessionBinding {
            version: SECURE_SESSION_VERSION,
            initiator: initiator_peer,
            responder: response.responder,
            responder_transport_certificate_sha256: response.responder_transport_certificate_sha256,
        };

        SecureSession::new(
            self.local_device.clone(),
            remote_offer.remote_device().clone(),
            Some(trusted_remote_transport_sha256),
            connection,
            DoubleRatchet::from_seed(bootstrap.seed),
            binding,
        )
    }
}

pub struct SessionResponder {
    local_device: Device,
    local_identity: IdentityKeyPair,
    prekey_store: LocalPrekeyStore,
    local_transport_certificate_sha256: [u8; 32],
}

impl SessionResponder {
    pub fn new(
        local_device: Device,
        local_identity: IdentityKeyPair,
        prekey_store: LocalPrekeyStore,
        local_transport_certificate_der: &[u8],
    ) -> Result<Self, MessagingError> {
        validate_local_device_identity(&local_device, &local_identity)?;

        let public_bundle = prekey_store.public_bundle();
        if public_bundle.identity != *local_device.identity_keys() {
            return Err(MessagingError::LocalDeviceIdentityMismatch);
        }

        Ok(Self {
            local_device,
            local_identity,
            prekey_store,
            local_transport_certificate_sha256: transport_certificate_sha256(
                local_transport_certificate_der,
            ),
        })
    }

    pub fn remote_session_offer(&self) -> Result<RemoteSessionOffer, MessagingError> {
        RemoteSessionOffer::from_parts(
            self.local_device.clone(),
            self.prekey_store.public_bundle(),
            self.local_transport_certificate_sha256,
        )
    }

    pub async fn accept(
        &mut self,
        connection: TransportConnection,
    ) -> Result<SecureSession, MessagingError> {
        let frame = connection.receive_frame().await?;
        let request_bytes = expect_handshake(frame, "expected handshake request")?;
        let request: SessionRequest = bincode::deserialize(&request_bytes)?;
        ensure_supported_version(request.version)?;

        let expected_local_peer = SessionPeer::from_device(&self.local_device);
        if request.expected_responder != expected_local_peer {
            return Err(MessagingError::RemoteBindingMismatch(
                "requested responder device metadata",
            ));
        }
        if request.expected_responder_transport_certificate_sha256
            != self.local_transport_certificate_sha256
        {
            return Err(MessagingError::TransportBindingMismatch);
        }
        if request.initiator.identity_keys != request.x3dh.initiator_identity {
            return Err(MessagingError::RemoteBindingMismatch(
                "initiator identity keys",
            ));
        }

        let initiator_device = request.initiator.clone().try_into_device()?;
        let responder_bootstrap =
            accept_session(&self.local_identity, &mut self.prekey_store, &request.x3dh)?;
        let responder_peer = SessionPeer::from_device(&self.local_device);

        let response = SessionResponse {
            version: SECURE_SESSION_VERSION,
            responder: responder_peer.clone(),
            responder_transport_certificate_sha256: self.local_transport_certificate_sha256,
            consumed_one_time_prekey_id: responder_bootstrap.consumed_one_time_prekey_id,
        };

        connection
            .send_frame(&TransportFrame::Handshake(bincode::serialize(&response)?))
            .await?;

        let binding = SessionBinding {
            version: SECURE_SESSION_VERSION,
            initiator: request.initiator,
            responder: responder_peer,
            responder_transport_certificate_sha256: self.local_transport_certificate_sha256,
        };

        SecureSession::new(
            self.local_device.clone(),
            initiator_device,
            None,
            connection,
            DoubleRatchet::from_seed(responder_bootstrap.seed),
            binding,
        )
    }
}

pub struct SecureSession {
    local_device: Device,
    remote_device: Device,
    remote_transport_certificate_sha256: Option<[u8; 32]>,
    connection: TransportConnection,
    ratchet: DoubleRatchet,
    session_id: [u8; 32],
    associated_data: Vec<u8>,
}

impl SecureSession {
    fn new(
        local_device: Device,
        remote_device: Device,
        remote_transport_certificate_sha256: Option<[u8; 32]>,
        connection: TransportConnection,
        ratchet: DoubleRatchet,
        binding: SessionBinding,
    ) -> Result<Self, MessagingError> {
        let associated_data = encode(&binding)?;
        let session_id = session_id_from_binding(&associated_data);

        Ok(Self {
            local_device,
            remote_device,
            remote_transport_certificate_sha256,
            connection,
            ratchet,
            session_id,
            associated_data,
        })
    }

    pub fn local_device(&self) -> &Device {
        &self.local_device
    }

    pub fn remote_device(&self) -> &Device {
        &self.remote_device
    }

    pub fn remote_transport_certificate_sha256(&self) -> Option<[u8; 32]> {
        self.remote_transport_certificate_sha256
    }

    pub fn role(&self) -> SessionRole {
        self.ratchet.role()
    }

    pub fn session_id(&self) -> &[u8; 32] {
        &self.session_id
    }

    pub fn forward_secrecy_state(&self) -> RatchetStateSnapshot {
        self.ratchet.state_snapshot()
    }

    pub async fn send_encrypted(&mut self, plaintext: &[u8]) -> Result<(), MessagingError> {
        let encrypted = self.ratchet.encrypt(plaintext, &self.associated_data)?;
        self.connection
            .send_frame(&TransportFrame::payload(bincode::serialize(&encrypted)?))
            .await?;
        Ok(())
    }

    pub async fn receive_encrypted(&mut self) -> Result<Vec<u8>, MessagingError> {
        let frame = self.connection.receive_frame().await?;
        let payload = match frame {
            TransportFrame::Payload(payload) => payload,
            _ => {
                return Err(MessagingError::UnexpectedFrame(
                    "expected encrypted payload frame",
                ));
            }
        };
        let encrypted: EncryptedMessage = bincode::deserialize(&payload)?;
        Ok(self.ratchet.decrypt(&encrypted, &self.associated_data)?)
    }

    pub fn close(&self, reason: &'static str) {
        self.connection.close(reason);
    }
}

fn validate_local_device_identity(
    local_device: &Device,
    local_identity: &IdentityKeyPair,
) -> Result<(), MessagingError> {
    if local_device.identity_keys() != &local_identity.public_keys() {
        return Err(MessagingError::LocalDeviceIdentityMismatch);
    }
    Ok(())
}

fn ensure_supported_version(version: u8) -> Result<(), MessagingError> {
    if version == SECURE_SESSION_VERSION {
        Ok(())
    } else {
        Err(MessagingError::InvalidHandshakeVersion(version))
    }
}

fn expect_handshake(
    frame: TransportFrame,
    unexpected_label: &'static str,
) -> Result<Vec<u8>, MessagingError> {
    match frame {
        TransportFrame::Handshake(payload) => Ok(payload),
        _ => Err(MessagingError::UnexpectedFrame(unexpected_label)),
    }
}

fn encode<T>(value: &T) -> Result<Vec<u8>, MessagingError>
where
    T: Serialize,
{
    Ok(bincode::serialize(value)?)
}

fn session_id_from_binding(associated_data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"localmessenger/secure-session/v1");
    hasher.update(associated_data);
    hasher.finalize().into()
}
