use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use rand_core::{CryptoRng, RngCore};

use localmessenger_server_protocol::{
    AuthChallenge, AuthHello, AuthOk, AuthResponse, DeviceRegistrationBundle,
    SERVER_PROTOCOL_VERSION, auth_challenge_payload,
};

#[derive(Debug, Clone)]
pub struct RegisteredDevice {
    pub member_id: String,
    pub device_id: String,
    pub device_name: String,
    pub auth_public_key: [u8; 32],
    pub disabled: bool,
}

impl RegisteredDevice {
    pub fn from_bundle(bundle: &DeviceRegistrationBundle) -> Self {
        Self {
            member_id: bundle.member_id.clone(),
            device_id: bundle.device_id.clone(),
            device_name: bundle.device_name.clone(),
            auth_public_key: bundle.auth_public_key,
            disabled: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthChallengeState {
    pub challenge: AuthChallenge,
    consumed: bool,
}

#[derive(Debug, Clone)]
pub struct Authenticator {
    challenge_ttl_ms: i64,
}

impl Authenticator {
    pub fn new(challenge_ttl_ms: i64) -> Self {
        Self { challenge_ttl_ms }
    }

    pub fn issue_challenge<R>(&self, rng: &mut R, now_unix_ms: i64) -> AuthChallengeState
    where
        R: RngCore + CryptoRng,
    {
        let mut nonce = [0_u8; 32];
        rng.fill_bytes(&mut nonce);
        AuthChallengeState {
            challenge: AuthChallenge {
                version: SERVER_PROTOCOL_VERSION,
                nonce,
                issued_at_unix_ms: now_unix_ms,
            },
            consumed: false,
        }
    }

    pub fn verify_response(
        &self,
        hello: &AuthHello,
        response: &AuthResponse,
        record: &RegisteredDevice,
        challenge: &mut AuthChallengeState,
        now_unix_ms: i64,
    ) -> Result<AuthOk, String> {
        if challenge.consumed {
            return Err("challenge already consumed".to_string());
        }
        if response.version != SERVER_PROTOCOL_VERSION {
            return Err(format!(
                "unsupported auth response version {}",
                response.version
            ));
        }
        if response.member_id != hello.member_id || response.device_id != hello.device_id {
            return Err("auth response identity does not match hello".to_string());
        }
        if hello.member_id != record.member_id || hello.device_id != record.device_id {
            return Err("registered device does not match hello".to_string());
        }
        if challenge.challenge.nonce != response.nonce {
            return Err("auth nonce mismatch".to_string());
        }
        if now_unix_ms.saturating_sub(challenge.challenge.issued_at_unix_ms) > self.challenge_ttl_ms
        {
            return Err("auth challenge expired".to_string());
        }
        if record.disabled {
            return Err("device is disabled".to_string());
        }

        let verifying_key = VerifyingKey::from_bytes(&record.auth_public_key)
            .map_err(|_| "invalid registered public key".to_string())?;
        let payload = auth_challenge_payload(&hello.member_id, &hello.device_id, &response.nonce);
        let signature_bytes: [u8; 64] = response
            .signature
            .as_slice()
            .try_into()
            .map_err(|_| "invalid auth signature length".to_string())?;
        let signature = Signature::from_bytes(&signature_bytes);
        verifying_key
            .verify(&payload, &signature)
            .map_err(|_| "invalid auth signature".to_string())?;

        challenge.consumed = true;

        Ok(AuthOk {
            version: SERVER_PROTOCOL_VERSION,
            member_id: hello.member_id.clone(),
            device_id: hello.device_id.clone(),
            device_name: record.device_name.clone(),
            server_time_unix_ms: now_unix_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use localmessenger_server_protocol::{
        AuthHello, AuthResponse, DeviceRegistrationBundle, SERVER_PROTOCOL_VERSION,
        auth_challenge_payload,
    };
    use rand_core::OsRng;

    use super::{Authenticator, RegisteredDevice};
    use localmessenger_core::{DeviceId, MemberId};
    use localmessenger_crypto::IdentityKeyPair;

    fn sample_bundle(identity: &IdentityKeyPair) -> DeviceRegistrationBundle {
        DeviceRegistrationBundle::new(
            &MemberId::new("alice").expect("member"),
            &DeviceId::new("alice-phone").expect("device"),
            "Alice Phone",
            identity.signing_public(),
        )
    }

    #[test]
    fn auth_accepts_valid_signature() {
        let mut rng = OsRng;
        let authenticator = Authenticator::new(30_000);
        let identity = IdentityKeyPair::generate(&mut rng);
        let bundle = sample_bundle(&identity);
        let record = RegisteredDevice::from_bundle(&bundle);
        let hello = AuthHello {
            version: SERVER_PROTOCOL_VERSION,
            member_id: bundle.member_id.clone(),
            device_id: bundle.device_id.clone(),
        };
        let mut challenge = authenticator.issue_challenge(&mut rng, 1_000);
        let response = AuthResponse {
            version: SERVER_PROTOCOL_VERSION,
            member_id: bundle.member_id.clone(),
            device_id: bundle.device_id.clone(),
            nonce: challenge.challenge.nonce,
            signature: identity
                .sign_message(&auth_challenge_payload(
                    &bundle.member_id,
                    &bundle.device_id,
                    &challenge.challenge.nonce,
                ))
                .to_vec(),
        };

        let ok = authenticator
            .verify_response(&hello, &response, &record, &mut challenge, 2_000)
            .expect("auth should verify");
        assert_eq!(ok.device_name, "Alice Phone");
    }

    #[test]
    fn auth_rejects_wrong_key_reuse_expiry_and_disabled_device() {
        let mut rng = OsRng;
        let authenticator = Authenticator::new(1_000);
        let identity = IdentityKeyPair::generate(&mut rng);
        let wrong_identity = IdentityKeyPair::generate(&mut rng);
        let bundle = sample_bundle(&identity);
        let hello = AuthHello {
            version: SERVER_PROTOCOL_VERSION,
            member_id: bundle.member_id.clone(),
            device_id: bundle.device_id.clone(),
        };

        let mut wrong_key_challenge = authenticator.issue_challenge(&mut rng, 10);
        let wrong_response = AuthResponse {
            version: SERVER_PROTOCOL_VERSION,
            member_id: bundle.member_id.clone(),
            device_id: bundle.device_id.clone(),
            nonce: wrong_key_challenge.challenge.nonce,
            signature: wrong_identity
                .sign_message(&auth_challenge_payload(
                    &bundle.member_id,
                    &bundle.device_id,
                    &wrong_key_challenge.challenge.nonce,
                ))
                .to_vec(),
        };
        let record = RegisteredDevice::from_bundle(&bundle);
        assert!(
            authenticator
                .verify_response(
                    &hello,
                    &wrong_response,
                    &record,
                    &mut wrong_key_challenge,
                    20
                )
                .is_err()
        );

        let mut reused_challenge = authenticator.issue_challenge(&mut rng, 10);
        let valid_response = AuthResponse {
            version: SERVER_PROTOCOL_VERSION,
            member_id: bundle.member_id.clone(),
            device_id: bundle.device_id.clone(),
            nonce: reused_challenge.challenge.nonce,
            signature: identity
                .sign_message(&auth_challenge_payload(
                    &bundle.member_id,
                    &bundle.device_id,
                    &reused_challenge.challenge.nonce,
                ))
                .to_vec(),
        };
        authenticator
            .verify_response(&hello, &valid_response, &record, &mut reused_challenge, 20)
            .expect("first use should work");
        assert!(
            authenticator
                .verify_response(&hello, &valid_response, &record, &mut reused_challenge, 20)
                .is_err()
        );

        let mut expired_challenge = authenticator.issue_challenge(&mut rng, 10);
        let expired_response = AuthResponse {
            version: SERVER_PROTOCOL_VERSION,
            member_id: bundle.member_id.clone(),
            device_id: bundle.device_id.clone(),
            nonce: expired_challenge.challenge.nonce,
            signature: identity
                .sign_message(&auth_challenge_payload(
                    &bundle.member_id,
                    &bundle.device_id,
                    &expired_challenge.challenge.nonce,
                ))
                .to_vec(),
        };
        assert!(
            authenticator
                .verify_response(
                    &hello,
                    &expired_response,
                    &record,
                    &mut expired_challenge,
                    2_500
                )
                .is_err()
        );

        let mut disabled_record = RegisteredDevice::from_bundle(&bundle);
        disabled_record.disabled = true;
        let mut disabled_challenge = authenticator.issue_challenge(&mut rng, 10);
        let disabled_response = AuthResponse {
            version: SERVER_PROTOCOL_VERSION,
            member_id: bundle.member_id.clone(),
            device_id: bundle.device_id.clone(),
            nonce: disabled_challenge.challenge.nonce,
            signature: identity
                .sign_message(&auth_challenge_payload(
                    &bundle.member_id,
                    &bundle.device_id,
                    &disabled_challenge.challenge.nonce,
                ))
                .to_vec(),
        };
        assert!(
            authenticator
                .verify_response(
                    &hello,
                    &disabled_response,
                    &disabled_record,
                    &mut disabled_challenge,
                    20,
                )
                .is_err()
        );
    }
}
