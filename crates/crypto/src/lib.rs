#![forbid(unsafe_code)]

mod error;
mod identity;
mod kdf;
mod prekey;
mod ratchet;
mod x3dh;

pub use error::CryptoError;
pub use identity::{IdentityKeyMaterial, IdentityKeyPair, IdentityPublicKeys};
pub use prekey::{
    LocalPrekeyStore, OneTimePrekeyMaterial, OneTimePrekeyPublic, PrekeyStoreMaterial,
    PublicPrekeyBundle, SignedPrekeyMaterial, SignedPrekeyPublic,
};
pub use ratchet::{
    DoubleRatchet, EncryptedMessage, MessageHeader, RatchetStateSnapshot, SessionRole, SessionSeed,
};
pub use x3dh::{
    InitiatorHandshake, InitiatorSession, ResponderSession, accept_session, initiate_session,
};

#[cfg(test)]
mod tests {
    use rand_core::OsRng;

    use crate::prekey::LocalPrekeyStore;
    use crate::ratchet::DoubleRatchet;
    use crate::{CryptoError, IdentityKeyPair, accept_session, initiate_session};

    #[test]
    fn signed_prekey_bundle_verifies_and_rejects_tampering() {
        let mut rng = OsRng;
        let identity = IdentityKeyPair::generate(&mut rng);
        let store = LocalPrekeyStore::generate(&mut rng, &identity, 7, 2, 100);
        let bundle = store.public_bundle();

        assert!(bundle.verify().is_ok());

        let mut tampered = bundle.clone();
        tampered.signed_prekey.signature[0] ^= 0xA5;
        let error = tampered.verify().expect_err("tampered signature must fail");
        assert!(matches!(error, CryptoError::InvalidSignature));
    }

    #[test]
    fn x3dh_session_seed_matches_on_both_sides() {
        let mut rng = OsRng;
        let alice = IdentityKeyPair::generate(&mut rng);
        let bob = IdentityKeyPair::generate(&mut rng);
        let mut bob_store = LocalPrekeyStore::generate(&mut rng, &bob, 21, 3, 300);
        let bob_bundle = bob_store.public_bundle();

        let alice_session = initiate_session(&mut rng, &alice, &bob_bundle)
            .expect("initiator session should be created");
        let bob_session = accept_session(&bob, &mut bob_store, &alice_session.handshake)
            .expect("responder session should be created");

        assert_eq!(alice_session.seed.root_key, bob_session.seed.root_key);
        assert_eq!(
            alice_session.seed.sending_chain_key,
            bob_session.seed.receiving_chain_key
        );
        assert_eq!(
            bob_session.consumed_one_time_prekey_id,
            alice_session.handshake.responder_one_time_prekey_id
        );
    }

    #[test]
    fn double_ratchet_round_trip_supports_out_of_order_messages() {
        let mut rng = OsRng;
        let alice = IdentityKeyPair::generate(&mut rng);
        let bob = IdentityKeyPair::generate(&mut rng);
        let mut bob_store = LocalPrekeyStore::generate(&mut rng, &bob, 31, 4, 500);
        let bob_bundle = bob_store.public_bundle();

        let alice_bootstrap = initiate_session(&mut rng, &alice, &bob_bundle)
            .expect("initiator bootstrap should succeed");
        let bob_bootstrap = accept_session(&bob, &mut bob_store, &alice_bootstrap.handshake)
            .expect("responder bootstrap should succeed");

        let mut alice_ratchet = DoubleRatchet::from_seed(alice_bootstrap.seed);
        let mut bob_ratchet = DoubleRatchet::from_seed(bob_bootstrap.seed);

        let ad = b"chat:room-1";
        let msg1 = alice_ratchet
            .encrypt(b"first", ad)
            .expect("first message should encrypt");
        let msg2 = alice_ratchet
            .encrypt(b"second", ad)
            .expect("second message should encrypt");

        let second = bob_ratchet
            .decrypt(&msg2, ad)
            .expect("receiver should handle skipped keys");
        let first = bob_ratchet
            .decrypt(&msg1, ad)
            .expect("receiver should decrypt delayed first message");

        assert_eq!(first, b"first");
        assert_eq!(second, b"second");

        let reply = bob_ratchet
            .encrypt(b"ack", ad)
            .expect("reply should encrypt after ratchet step");
        let reply_plaintext = alice_ratchet
            .decrypt(&reply, ad)
            .expect("initiator should decrypt ratcheted reply");
        assert_eq!(reply_plaintext, b"ack");

        let follow_up = alice_ratchet
            .encrypt(b"after-ratchet", ad)
            .expect("initiator follow-up should encrypt");
        let follow_up_plaintext = bob_ratchet
            .decrypt(&follow_up, ad)
            .expect("responder should decrypt next ratchet message");
        assert_eq!(follow_up_plaintext, b"after-ratchet");
    }

    #[test]
    fn ratchet_state_snapshot_changes_after_remote_ratchet_step() {
        let mut rng = OsRng;
        let alice = IdentityKeyPair::generate(&mut rng);
        let bob = IdentityKeyPair::generate(&mut rng);
        let mut bob_store = LocalPrekeyStore::generate(&mut rng, &bob, 41, 4, 700);
        let bob_bundle = bob_store.public_bundle();

        let alice_bootstrap = initiate_session(&mut rng, &alice, &bob_bundle)
            .expect("initiator bootstrap should succeed");
        let bob_bootstrap = accept_session(&bob, &mut bob_store, &alice_bootstrap.handshake)
            .expect("responder bootstrap should succeed");

        let mut alice_ratchet = DoubleRatchet::from_seed(alice_bootstrap.seed);
        let mut bob_ratchet = DoubleRatchet::from_seed(bob_bootstrap.seed);

        let before = alice_ratchet.state_snapshot();
        let ad = b"chat:security";

        let outbound = alice_ratchet
            .encrypt(b"probe", ad)
            .expect("probe should encrypt");
        bob_ratchet
            .decrypt(&outbound, ad)
            .expect("bob should decrypt probe");
        let reply = bob_ratchet
            .encrypt(b"ack", ad)
            .expect("reply should encrypt");
        alice_ratchet
            .decrypt(&reply, ad)
            .expect("alice should decrypt reply");

        let after = alice_ratchet.state_snapshot();
        assert_ne!(before.local_ratchet_public(), after.local_ratchet_public());
        assert_ne!(
            before.remote_ratchet_public(),
            after.remote_ratchet_public()
        );
        assert_eq!(after.skipped_message_key_count(), 0);
    }

    #[test]
    fn key_material_round_trip_restores_identity_and_prekeys() {
        let mut rng = OsRng;
        let identity = IdentityKeyPair::generate(&mut rng);
        let store = LocalPrekeyStore::generate(&mut rng, &identity, 71, 3, 900);

        let restored_identity = IdentityKeyPair::from_material(&identity.to_material());
        let restored_store =
            LocalPrekeyStore::from_material(store.to_material()).expect("prekeys should restore");

        assert_eq!(restored_identity.public_keys(), identity.public_keys());
        assert_eq!(restored_store.public_bundle(), store.public_bundle());
    }
}
