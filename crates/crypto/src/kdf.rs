use hkdf::Hkdf;
use x25519_dalek::{PublicKey, StaticSecret};

use crate::error::CryptoError;

pub(crate) fn diffie_hellman(secret: &StaticSecret, remote_public: &[u8; 32]) -> [u8; 32] {
    let remote_public = PublicKey::from(*remote_public);
    secret.diffie_hellman(&remote_public).to_bytes()
}

pub(crate) fn hkdf_expand<const N: usize>(
    salt: Option<&[u8]>,
    ikm: &[u8],
    info: &[u8],
) -> Result<[u8; N], CryptoError> {
    let hkdf = Hkdf::<sha2::Sha256>::new(salt, ikm);
    let mut output = [0_u8; N];
    hkdf.expand(info, &mut output)
        .map_err(|_| CryptoError::InvalidKeyMaterial("hkdf"))?;
    Ok(output)
}

pub(crate) fn derive_initial_root_key(material: &[u8]) -> Result<[u8; 32], CryptoError> {
    hkdf_expand(None, material, b"localmessenger/x3dh/root")
}

pub(crate) fn root_kdf(
    root_key: &[u8; 32],
    dh_output: &[u8; 32],
) -> Result<([u8; 32], [u8; 32]), CryptoError> {
    let output = hkdf_expand::<64>(
        Some(root_key),
        dh_output,
        b"localmessenger/double-ratchet/root",
    )?;
    let mut next_root = [0_u8; 32];
    let mut next_chain = [0_u8; 32];
    next_root.copy_from_slice(&output[..32]);
    next_chain.copy_from_slice(&output[32..]);
    Ok((next_root, next_chain))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MessageKeyMaterial {
    pub cipher_key: [u8; 32],
    pub nonce: [u8; 12],
}

pub(crate) fn chain_kdf(
    chain_key: &[u8; 32],
) -> Result<([u8; 32], MessageKeyMaterial), CryptoError> {
    let output = hkdf_expand::<76>(
        Some(chain_key),
        b"step",
        b"localmessenger/double-ratchet/chain",
    )?;
    let mut next_chain_key = [0_u8; 32];
    let mut cipher_key = [0_u8; 32];
    let mut nonce = [0_u8; 12];

    next_chain_key.copy_from_slice(&output[..32]);
    cipher_key.copy_from_slice(&output[32..64]);
    nonce.copy_from_slice(&output[64..]);

    Ok((next_chain_key, MessageKeyMaterial { cipher_key, nonce }))
}
