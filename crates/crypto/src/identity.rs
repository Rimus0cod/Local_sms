use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use x25519_dalek::{PublicKey, StaticSecret};

use crate::error::CryptoError;

const SIGNED_PREKEY_CONTEXT: &[u8] = b"localmessenger/signed-prekey/v1";

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityKeyMaterial {
    pub agreement_secret_key: [u8; 32],
    pub signing_secret_key: [u8; 32],
}

pub struct IdentityKeyPair {
    agreement_secret: StaticSecret,
    agreement_public: PublicKey,
    signing_key: SigningKey,
}

impl IdentityKeyPair {
    pub fn generate<R>(rng: &mut R) -> Self
    where
        R: RngCore + CryptoRng,
    {
        let agreement_secret = StaticSecret::random_from_rng(&mut *rng);
        let agreement_public = PublicKey::from(&agreement_secret);
        let mut signing_seed = [0_u8; 32];
        rng.fill_bytes(&mut signing_seed);
        let signing_key = SigningKey::from_bytes(&signing_seed);

        Self {
            agreement_secret,
            agreement_public,
            signing_key,
        }
    }

    pub fn agreement_secret(&self) -> &StaticSecret {
        &self.agreement_secret
    }

    pub fn agreement_public(&self) -> [u8; 32] {
        self.agreement_public.to_bytes()
    }

    pub fn signing_public(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    pub fn public_keys(&self) -> IdentityPublicKeys {
        IdentityPublicKeys {
            agreement_public_key: self.agreement_public(),
            signing_public_key: self.signing_public(),
        }
    }

    pub fn to_material(&self) -> IdentityKeyMaterial {
        IdentityKeyMaterial {
            agreement_secret_key: self.agreement_secret.to_bytes(),
            signing_secret_key: self.signing_key.to_bytes(),
        }
    }

    pub fn from_material(material: &IdentityKeyMaterial) -> Self {
        let agreement_secret = StaticSecret::from(material.agreement_secret_key);
        let agreement_public = PublicKey::from(&agreement_secret);
        let signing_key = SigningKey::from_bytes(&material.signing_secret_key);

        Self {
            agreement_secret,
            agreement_public,
            signing_key,
        }
    }

    pub fn sign_signed_prekey(&self, prekey_id: u32, prekey_public: &[u8; 32]) -> [u8; 64] {
        let payload = signed_prekey_payload(prekey_id, prekey_public);
        self.signing_key.sign(&payload).to_bytes()
    }

    pub fn sign_message(&self, payload: &[u8]) -> [u8; 64] {
        self.signing_key.sign(payload).to_bytes()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityPublicKeys {
    pub agreement_public_key: [u8; 32],
    pub signing_public_key: [u8; 32],
}

impl IdentityPublicKeys {
    pub fn verify_signed_prekey(
        &self,
        prekey_id: u32,
        prekey_public: &[u8; 32],
        signature: &[u8],
    ) -> Result<(), CryptoError> {
        let verifying_key = VerifyingKey::from_bytes(&self.signing_public_key)
            .map_err(|_| CryptoError::InvalidSignature)?;
        let payload = signed_prekey_payload(prekey_id, prekey_public);
        let signature: [u8; 64] = signature
            .try_into()
            .map_err(|_| CryptoError::InvalidSignature)?;
        let signature = Signature::from_bytes(&signature);
        verifying_key
            .verify(&payload, &signature)
            .map_err(|_| CryptoError::InvalidSignature)
    }

    pub fn verify_message(&self, payload: &[u8], signature: &[u8]) -> Result<(), CryptoError> {
        let verifying_key = VerifyingKey::from_bytes(&self.signing_public_key)
            .map_err(|_| CryptoError::InvalidSignature)?;
        let signature: [u8; 64] = signature
            .try_into()
            .map_err(|_| CryptoError::InvalidSignature)?;
        let signature = Signature::from_bytes(&signature);
        verifying_key
            .verify(payload, &signature)
            .map_err(|_| CryptoError::InvalidSignature)
    }
}

fn signed_prekey_payload(prekey_id: u32, prekey_public: &[u8; 32]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(SIGNED_PREKEY_CONTEXT.len() + 4 + prekey_public.len());
    payload.extend_from_slice(SIGNED_PREKEY_CONTEXT);
    payload.extend_from_slice(&prekey_id.to_be_bytes());
    payload.extend_from_slice(prekey_public);
    payload
}
