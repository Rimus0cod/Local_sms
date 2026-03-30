use std::collections::BTreeMap;

use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use x25519_dalek::{PublicKey, StaticSecret};

use crate::error::CryptoError;
use crate::identity::{IdentityKeyPair, IdentityPublicKeys};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedPrekeyMaterial {
    pub id: u32,
    pub secret_key: [u8; 32],
    pub signature: Vec<u8>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OneTimePrekeyMaterial {
    pub id: u32,
    pub secret_key: [u8; 32],
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrekeyStoreMaterial {
    pub identity: IdentityPublicKeys,
    pub signed_prekey: SignedPrekeyMaterial,
    pub one_time_prekeys: Vec<OneTimePrekeyMaterial>,
}

pub struct SignedPrekeyRecord {
    id: u32,
    secret: StaticSecret,
    public: [u8; 32],
    signature: [u8; 64],
}

impl SignedPrekeyRecord {
    pub fn generate<R>(rng: &mut R, identity: &IdentityKeyPair, id: u32) -> Self
    where
        R: RngCore + CryptoRng,
    {
        let secret = StaticSecret::random_from_rng(&mut *rng);
        let public = PublicKey::from(&secret).to_bytes();
        let signature = identity.sign_signed_prekey(id, &public);

        Self {
            id,
            secret,
            public,
            signature,
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn secret(&self) -> &StaticSecret {
        &self.secret
    }

    pub fn public_key(&self) -> [u8; 32] {
        self.public
    }

    pub fn public_record(&self) -> SignedPrekeyPublic {
        SignedPrekeyPublic {
            id: self.id,
            public_key: self.public,
            signature: self.signature.to_vec(),
        }
    }

    fn to_material(&self) -> SignedPrekeyMaterial {
        SignedPrekeyMaterial {
            id: self.id,
            secret_key: self.secret.to_bytes(),
            signature: self.signature.to_vec(),
        }
    }

    fn from_material(material: SignedPrekeyMaterial) -> Result<Self, CryptoError> {
        let signature: [u8; 64] = material
            .signature
            .try_into()
            .map_err(|_| CryptoError::InvalidSignature)?;
        let secret = StaticSecret::from(material.secret_key);
        let public = PublicKey::from(&secret).to_bytes();

        Ok(Self {
            id: material.id,
            secret,
            public,
            signature,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedPrekeyPublic {
    pub id: u32,
    pub public_key: [u8; 32],
    pub signature: Vec<u8>,
}

pub struct OneTimePrekeyRecord {
    id: u32,
    secret: StaticSecret,
    public: [u8; 32],
}

impl OneTimePrekeyRecord {
    pub fn generate<R>(rng: &mut R, id: u32) -> Self
    where
        R: RngCore + CryptoRng,
    {
        let secret = StaticSecret::random_from_rng(&mut *rng);
        let public = PublicKey::from(&secret).to_bytes();
        Self { id, secret, public }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn secret(&self) -> &StaticSecret {
        &self.secret
    }

    pub fn public_record(&self) -> OneTimePrekeyPublic {
        OneTimePrekeyPublic {
            id: self.id,
            public_key: self.public,
        }
    }

    fn to_material(&self) -> OneTimePrekeyMaterial {
        OneTimePrekeyMaterial {
            id: self.id,
            secret_key: self.secret.to_bytes(),
        }
    }

    fn from_material(material: OneTimePrekeyMaterial) -> Self {
        let secret = StaticSecret::from(material.secret_key);
        let public = PublicKey::from(&secret).to_bytes();

        Self {
            id: material.id,
            secret,
            public,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OneTimePrekeyPublic {
    pub id: u32,
    pub public_key: [u8; 32],
}

pub struct LocalPrekeyStore {
    identity: IdentityPublicKeys,
    signed_prekey: SignedPrekeyRecord,
    one_time_prekeys: BTreeMap<u32, OneTimePrekeyRecord>,
}

impl LocalPrekeyStore {
    pub fn generate<R>(
        rng: &mut R,
        identity: &IdentityKeyPair,
        signed_prekey_id: u32,
        one_time_prekey_count: usize,
        one_time_prekey_id_start: u32,
    ) -> Self
    where
        R: RngCore + CryptoRng,
    {
        let signed_prekey = SignedPrekeyRecord::generate(rng, identity, signed_prekey_id);
        let mut one_time_prekeys = BTreeMap::new();
        for offset in 0..one_time_prekey_count {
            let id = one_time_prekey_id_start + offset as u32;
            let record = OneTimePrekeyRecord::generate(rng, id);
            one_time_prekeys.insert(id, record);
        }

        Self {
            identity: identity.public_keys(),
            signed_prekey,
            one_time_prekeys,
        }
    }

    pub fn signed_prekey(&self) -> &SignedPrekeyRecord {
        &self.signed_prekey
    }

    pub fn identity(&self) -> &IdentityPublicKeys {
        &self.identity
    }

    pub fn take_one_time_prekey(
        &mut self,
        prekey_id: u32,
    ) -> Result<OneTimePrekeyRecord, CryptoError> {
        self.one_time_prekeys
            .remove(&prekey_id)
            .ok_or(CryptoError::MissingOneTimePrekey(prekey_id))
    }

    pub fn public_bundle(&self) -> PublicPrekeyBundle {
        PublicPrekeyBundle {
            identity: self.identity.clone(),
            signed_prekey: self.signed_prekey.public_record(),
            one_time_prekeys: self
                .one_time_prekeys
                .values()
                .map(OneTimePrekeyRecord::public_record)
                .collect(),
        }
    }

    pub fn to_material(&self) -> PrekeyStoreMaterial {
        PrekeyStoreMaterial {
            identity: self.identity.clone(),
            signed_prekey: self.signed_prekey.to_material(),
            one_time_prekeys: self
                .one_time_prekeys
                .values()
                .map(OneTimePrekeyRecord::to_material)
                .collect(),
        }
    }

    pub fn from_material(material: PrekeyStoreMaterial) -> Result<Self, CryptoError> {
        let one_time_prekeys = material
            .one_time_prekeys
            .into_iter()
            .map(OneTimePrekeyRecord::from_material)
            .map(|record| (record.id(), record))
            .collect();

        Ok(Self {
            identity: material.identity,
            signed_prekey: SignedPrekeyRecord::from_material(material.signed_prekey)?,
            one_time_prekeys,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicPrekeyBundle {
    pub identity: IdentityPublicKeys,
    pub signed_prekey: SignedPrekeyPublic,
    pub one_time_prekeys: Vec<OneTimePrekeyPublic>,
}

impl PublicPrekeyBundle {
    pub fn verify(&self) -> Result<(), CryptoError> {
        self.identity.verify_signed_prekey(
            self.signed_prekey.id,
            &self.signed_prekey.public_key,
            &self.signed_prekey.signature,
        )
    }

    pub fn first_one_time_prekey(&self) -> Option<&OneTimePrekeyPublic> {
        self.one_time_prekeys.first()
    }
}
