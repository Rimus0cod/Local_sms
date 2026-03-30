use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use x25519_dalek::{PublicKey, StaticSecret};

use crate::error::CryptoError;
use crate::identity::{IdentityKeyPair, IdentityPublicKeys};
use crate::kdf::{derive_initial_root_key, diffie_hellman, root_kdf};
use crate::prekey::{LocalPrekeyStore, PublicPrekeyBundle};
use crate::ratchet::{SessionRole, SessionSeed};

const X3DH_PREFIX: [u8; 32] = [0xFF; 32];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InitiatorHandshake {
    pub initiator_identity: IdentityPublicKeys,
    pub initiator_ephemeral_public_key: [u8; 32],
    pub initiator_ratchet_public_key: [u8; 32],
    pub responder_signed_prekey_id: u32,
    pub responder_one_time_prekey_id: Option<u32>,
}

pub struct InitiatorSession {
    pub handshake: InitiatorHandshake,
    pub seed: SessionSeed,
}

pub struct ResponderSession {
    pub seed: SessionSeed,
    pub consumed_one_time_prekey_id: Option<u32>,
}

pub fn initiate_session<R>(
    rng: &mut R,
    initiator_identity: &IdentityKeyPair,
    responder_bundle: &PublicPrekeyBundle,
) -> Result<InitiatorSession, CryptoError>
where
    R: RngCore + CryptoRng,
{
    responder_bundle.verify()?;

    let responder_signed_prekey_public = responder_bundle.signed_prekey.public_key;
    let responder_one_time_prekey_public = responder_bundle
        .first_one_time_prekey()
        .map(|prekey| (prekey.id, prekey.public_key));

    let ephemeral_secret = StaticSecret::random_from_rng(&mut *rng);
    let ephemeral_public = PublicKey::from(&ephemeral_secret).to_bytes();

    let ik_spk = diffie_hellman(
        initiator_identity.agreement_secret(),
        &responder_signed_prekey_public,
    );
    let ek_ik = diffie_hellman(
        &ephemeral_secret,
        &responder_bundle.identity.agreement_public_key,
    );
    let ek_spk = diffie_hellman(&ephemeral_secret, &responder_signed_prekey_public);

    let mut material = Vec::with_capacity(32 * 5);
    material.extend_from_slice(&X3DH_PREFIX);
    material.extend_from_slice(&ik_spk);
    material.extend_from_slice(&ek_ik);
    material.extend_from_slice(&ek_spk);

    if let Some((_, one_time_public)) = responder_one_time_prekey_public {
        let ek_opk = diffie_hellman(&ephemeral_secret, &one_time_public);
        material.extend_from_slice(&ek_opk);
    }

    let initial_root_key = derive_initial_root_key(&material)?;

    let initiator_ratchet_secret = StaticSecret::random_from_rng(&mut *rng);
    let initiator_ratchet_public = PublicKey::from(&initiator_ratchet_secret).to_bytes();
    let ratchet_dh = diffie_hellman(&initiator_ratchet_secret, &responder_signed_prekey_public);
    let (root_key, sending_chain_key) = root_kdf(&initial_root_key, &ratchet_dh)?;

    let handshake = InitiatorHandshake {
        initiator_identity: initiator_identity.public_keys(),
        initiator_ephemeral_public_key: ephemeral_public,
        initiator_ratchet_public_key: initiator_ratchet_public,
        responder_signed_prekey_id: responder_bundle.signed_prekey.id,
        responder_one_time_prekey_id: responder_one_time_prekey_public.map(|(id, _)| id),
    };

    Ok(InitiatorSession {
        handshake,
        seed: SessionSeed {
            role: SessionRole::Initiator,
            root_key,
            local_ratchet_secret: initiator_ratchet_secret,
            local_ratchet_public: initiator_ratchet_public,
            remote_ratchet_public: responder_signed_prekey_public,
            sending_chain_key: Some(sending_chain_key),
            receiving_chain_key: None,
        },
    })
}

pub fn accept_session(
    responder_identity: &IdentityKeyPair,
    prekey_store: &mut LocalPrekeyStore,
    handshake: &InitiatorHandshake,
) -> Result<ResponderSession, CryptoError> {
    let signed_prekey = prekey_store.signed_prekey();
    if signed_prekey.id() != handshake.responder_signed_prekey_id {
        return Err(CryptoError::MissingSignedPrekey(
            handshake.responder_signed_prekey_id,
        ));
    }
    let signed_prekey_secret = StaticSecret::from(signed_prekey.secret().to_bytes());
    let signed_prekey_public = signed_prekey.public_key();

    let initiator_ephemeral = handshake.initiator_ephemeral_public_key;

    let ik_spk = diffie_hellman(
        &signed_prekey_secret,
        &handshake.initiator_identity.agreement_public_key,
    );
    let ek_ik = diffie_hellman(responder_identity.agreement_secret(), &initiator_ephemeral);
    let ek_spk = diffie_hellman(&signed_prekey_secret, &initiator_ephemeral);

    let mut material = Vec::with_capacity(32 * 5);
    material.extend_from_slice(&X3DH_PREFIX);
    material.extend_from_slice(&ik_spk);
    material.extend_from_slice(&ek_ik);
    material.extend_from_slice(&ek_spk);

    let mut consumed_one_time_prekey_id = None;
    if let Some(prekey_id) = handshake.responder_one_time_prekey_id {
        let one_time_prekey = prekey_store.take_one_time_prekey(prekey_id)?;
        let ek_opk = diffie_hellman(one_time_prekey.secret(), &initiator_ephemeral);
        material.extend_from_slice(&ek_opk);
        consumed_one_time_prekey_id = Some(one_time_prekey.id());
    }

    let initial_root_key = derive_initial_root_key(&material)?;
    let receive_dh = diffie_hellman(
        &signed_prekey_secret,
        &handshake.initiator_ratchet_public_key,
    );
    let (root_key, receiving_chain_key) = root_kdf(&initial_root_key, &receive_dh)?;

    Ok(ResponderSession {
        seed: SessionSeed {
            role: SessionRole::Responder,
            root_key,
            local_ratchet_secret: signed_prekey_secret,
            local_ratchet_public: signed_prekey_public,
            remote_ratchet_public: handshake.initiator_ratchet_public_key,
            sending_chain_key: None,
            receiving_chain_key: Some(receiving_chain_key),
        },
        consumed_one_time_prekey_id,
    })
}
