use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use localmessenger_server_protocol::{
    InviteClaims, JoinAccepted, JoinWithInvite, SERVER_PROTOCOL_VERSION, decode_invite_certificate,
    encode_invite_link, verify_invite_link,
};

use crate::registry::RegistryDatabase;

#[derive(Clone)]
pub struct InviteService {
    registry: RegistryDatabase,
    secret: Vec<u8>,
}

impl InviteService {
    pub fn new(registry: RegistryDatabase, secret: impl Into<Vec<u8>>) -> Self {
        Self {
            registry,
            secret: secret.into(),
        }
    }

    pub async fn create_invite(
        &self,
        invite_id: String,
        label: String,
        server_addr: String,
        server_name: String,
        server_certificate_der: Vec<u8>,
        issued_at_unix_ms: i64,
        expires_at_unix_ms: i64,
        max_uses: u32,
    ) -> Result<String, String> {
        let claims = InviteClaims {
            version: SERVER_PROTOCOL_VERSION,
            invite_id,
            label,
            server_addr,
            server_name,
            server_certificate_der_base64: URL_SAFE_NO_PAD.encode(server_certificate_der),
            issued_at_unix_ms,
            expires_at_unix_ms,
            max_uses,
        };
        self.registry
            .create_invite(&claims, issued_at_unix_ms)
            .await?;
        encode_invite_link(&self.secret, &claims)
    }

    pub async fn join_with_invite(
        &self,
        request: &JoinWithInvite,
        now_unix_ms: i64,
    ) -> Result<JoinAccepted, String> {
        request.registration.validate()?;
        let claims = verify_invite_link(&self.secret, &request.invite_link)?;
        let Some(stored_invite) = self.registry.invite(&claims.invite_id).await? else {
            return Err("invite not found".to_string());
        };

        if stored_invite.status != "active" {
            return Err("invite is not active".to_string());
        }
        if now_unix_ms > stored_invite.expires_at_unix_ms {
            return Err("invite expired".to_string());
        }
        if stored_invite.used_count >= stored_invite.max_uses {
            return Err("invite usage limit reached".to_string());
        }

        self.registry
            .register_device(&request.registration, now_unix_ms)
            .await?;
        self.registry.mark_invite_used(&claims.invite_id).await?;

        Ok(JoinAccepted {
            version: SERVER_PROTOCOL_VERSION,
            invite_id: claims.invite_id,
            member_id: request.registration.member_id.clone(),
            device_id: request.registration.device_id.clone(),
            server_addr: claims.server_addr,
            server_name: claims.server_name,
            server_certificate_der_base64: claims.server_certificate_der_base64,
        })
    }

    pub fn invite_certificate_der(&self, invite_link: &str) -> Result<Vec<u8>, String> {
        let claims = verify_invite_link(&self.secret, invite_link)?;
        decode_invite_certificate(&claims)
    }
}

#[cfg(test)]
mod tests {
    use super::InviteService;
    use crate::registry::RegistryDatabase;
    use localmessenger_core::{DeviceId, MemberId};
    use localmessenger_server_protocol::{DeviceRegistrationBundle, JoinWithInvite};

    #[tokio::test]
    async fn invite_service_creates_and_consumes_signed_invites() {
        let registry = RegistryDatabase::open(":memory:").await.expect("db");
        let service = InviteService::new(registry.clone(), b"secret".to_vec());
        let link = service
            .create_invite(
                "inv-1".to_string(),
                "Home relay".to_string(),
                "127.0.0.1:7443".to_string(),
                "relay.local".to_string(),
                vec![1, 2, 3],
                10,
                100,
                1,
            )
            .await
            .expect("create");

        let accepted = service
            .join_with_invite(
                &JoinWithInvite {
                    invite_link: link.clone(),
                    registration: DeviceRegistrationBundle::new(
                        &MemberId::new("alice").expect("member"),
                        &DeviceId::new("alice-phone").expect("device"),
                        "Alice Phone",
                        [8_u8; 32],
                    ),
                },
                20,
            )
            .await
            .expect("join");
        assert_eq!(accepted.invite_id, "inv-1");
        assert_eq!(
            service.invite_certificate_der(&link).expect("certificate"),
            vec![1, 2, 3]
        );
        assert!(
            service
                .join_with_invite(
                    &JoinWithInvite {
                        invite_link: link,
                        registration: DeviceRegistrationBundle::new(
                            &MemberId::new("bob").expect("member"),
                            &DeviceId::new("bob-phone").expect("device"),
                            "Bob Phone",
                            [9_u8; 32],
                        ),
                    },
                    30,
                )
                .await
                .is_err()
        );
    }
}
