#![forbid(unsafe_code)]

mod device;
mod error;
mod ids;
mod member;
mod verification;

pub use device::Device;
pub use error::CoreError;
pub use ids::{DeviceId, MemberId};
pub use member::MemberProfile;
pub use verification::{
    DeviceVerificationQr, SafetyNumber, VerificationMethod, VerificationStatus,
};

#[cfg(test)]
mod tests {
    use localmessenger_crypto::IdentityKeyPair;
    use rand_core::OsRng;

    use crate::{
        CoreError, Device, DeviceId, DeviceVerificationQr, MemberId, MemberProfile, SafetyNumber,
        VerificationMethod, VerificationStatus,
    };

    #[test]
    fn member_profile_supports_multiple_devices() {
        let mut rng = OsRng;
        let member_id = MemberId::new("alice").expect("member id should be valid");
        let mut member = MemberProfile::new(member_id.clone(), "Alice")
            .expect("member profile should be created");

        let laptop = Device::from_identity_keypair(
            DeviceId::new("alice-laptop").expect("device id should be valid"),
            member_id.clone(),
            "Alice Laptop",
            &IdentityKeyPair::generate(&mut rng),
        )
        .expect("device should be created");
        let phone = Device::from_identity_keypair(
            DeviceId::new("alice-phone").expect("device id should be valid"),
            member_id,
            "Alice Phone",
            &IdentityKeyPair::generate(&mut rng),
        )
        .expect("device should be created");

        member.add_device(laptop).expect("first device should fit");
        member.add_device(phone).expect("second device should fit");

        assert_eq!(member.devices().count(), 2);
        assert!(member.verified_devices().is_empty());
    }

    #[test]
    fn safety_number_is_symmetric_and_can_verify_device() {
        let mut rng = OsRng;
        let alice_device = Device::from_identity_keypair(
            DeviceId::new("alice-phone").expect("device id should be valid"),
            MemberId::new("alice").expect("member id should be valid"),
            "Alice Phone",
            &IdentityKeyPair::generate(&mut rng),
        )
        .expect("device should be created");
        let mut bob_device = Device::from_identity_keypair(
            DeviceId::new("bob-phone").expect("device id should be valid"),
            MemberId::new("bob").expect("member id should be valid"),
            "Bob Phone",
            &IdentityKeyPair::generate(&mut rng),
        )
        .expect("device should be created");

        let from_alice = alice_device.safety_number_with(&bob_device);
        let from_bob = bob_device.safety_number_with(&alice_device);

        assert_eq!(from_alice, from_bob);
        assert!(!from_alice.digits().is_empty());

        bob_device
            .verify_with_safety_number(&alice_device, &from_alice)
            .expect("matching safety number should verify");

        assert_eq!(
            bob_device.verification_status(),
            &VerificationStatus::Verified {
                method: VerificationMethod::SafetyNumber,
            }
        );
    }

    #[test]
    fn qr_verification_marks_matching_device_as_verified() {
        let mut rng = OsRng;
        let member_id = MemberId::new("bob").expect("member id should be valid");
        let mut member =
            MemberProfile::new(member_id.clone(), "Bob").expect("member profile should exist");
        let device_id = DeviceId::new("bob-laptop").expect("device id should be valid");

        let device = Device::from_identity_keypair(
            device_id.clone(),
            member_id,
            "Bob Laptop",
            &IdentityKeyPair::generate(&mut rng),
        )
        .expect("device should be created");
        let qr_payload = device
            .qr_payload(None)
            .expect("QR payload generation should work");

        member.add_device(device).expect("device should be stored");
        member
            .verify_device_by_qr(&device_id, &qr_payload)
            .expect("matching QR should verify");

        let stored = member.device(&device_id).expect("device should exist");
        assert!(stored.is_verified());
        assert_eq!(
            stored.verification_status(),
            &VerificationStatus::Verified {
                method: VerificationMethod::QrCode,
            }
        );
    }

    #[test]
    fn qr_verification_rejects_tampered_payload() {
        let mut rng = OsRng;
        let mut device = Device::from_identity_keypair(
            DeviceId::new("eve-phone").expect("device id should be valid"),
            MemberId::new("eve").expect("member id should be valid"),
            "Eve Phone",
            &IdentityKeyPair::generate(&mut rng),
        )
        .expect("device should be created");

        let mut payload = DeviceVerificationQr::from_device(&device, None);
        payload.device_id = "wrong-device".to_string();
        let payload_bytes = payload.encode().expect("payload encoding should work");

        let error = device
            .verify_with_qr_payload(&payload_bytes)
            .expect_err("tampered QR must be rejected");
        assert!(matches!(error, CoreError::QrPayloadMismatch));
    }

    #[test]
    fn member_level_safety_verification_fails_on_wrong_number() {
        let mut rng = OsRng;
        let alice_device = Device::from_identity_keypair(
            DeviceId::new("alice-main").expect("device id should be valid"),
            MemberId::new("alice").expect("member id should be valid"),
            "Alice Main",
            &IdentityKeyPair::generate(&mut rng),
        )
        .expect("device should be created");

        let bob_member_id = MemberId::new("bob").expect("member id should be valid");
        let bob_device_id = DeviceId::new("bob-main").expect("device id should be valid");
        let bob_device = Device::from_identity_keypair(
            bob_device_id.clone(),
            bob_member_id.clone(),
            "Bob Main",
            &IdentityKeyPair::generate(&mut rng),
        )
        .expect("device should be created");

        let mut bob =
            MemberProfile::new(bob_member_id, "Bob").expect("member profile should be created");
        bob.add_device(bob_device).expect("device should be added");

        let wrong_safety = SafetyNumber::between(&alice_device, &alice_device);
        let error = bob
            .verify_device_by_safety_number(&bob_device_id, &alice_device, &wrong_safety)
            .expect_err("wrong safety number must fail");

        assert!(matches!(error, CoreError::SafetyNumberMismatch));
    }
}
