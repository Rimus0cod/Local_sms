pub mod config;
pub mod domain;
pub mod invite;
pub mod security;
pub mod workflow;

pub use config::{GroupConfig, GroupPolicy, TransportMode};
pub use domain::{
    AttachmentKind, AttachmentMeta, ChatMessage, DeviceId, DeviceProfile, GroupId, GroupRoster,
    MemberId, MemberProfile, MessageId, MessageKind, MessageReaction, Platform, PresenceState,
};
pub use invite::{InviteToken, InviteTransport};
pub use security::{
    CryptoProfile, RekeyPlan, RekeyReason, SafetyChecklist, VerificationState,
};
pub use workflow::{mvp_blueprint, next_milestones, ProjectBlueprint};

#[cfg(test)]
mod tests {
    use super::config::GroupPolicy;
    use super::domain::{
        AttachmentKind, AttachmentMeta, ChatMessage, DeviceId, DeviceProfile, GroupId, GroupRoster,
        MemberId, MemberProfile, MessageId, MessageKind, Platform, PresenceState, RosterError,
    };
    use super::invite::{InviteError, InviteToken, InviteTransport};
    use super::security::VerificationState;
    use std::time::{Duration, SystemTime};

    #[test]
    fn mvp_policy_respects_eight_member_limit() {
        let policy = GroupPolicy::mvp();
        assert_eq!(policy.max_members, 8);
        assert!(policy.validate().is_ok());
    }

    #[test]
    fn roster_rejects_ninth_member() {
        let policy = GroupPolicy::mvp();
        let owner = sample_member("owner", "Owner", "owner-laptop");
        let mut roster = GroupRoster::new(owner);

        for index in 1..policy.max_members {
            let member = sample_member(
                &format!("member_{index}"),
                &format!("Member {index}"),
                &format!("device_{index}"),
            );
            roster.add_member(member, &policy).expect("member should fit");
        }

        let overflow = sample_member("member_overflow", "Overflow", "device_overflow");
        let error = roster
            .add_member(overflow, &policy)
            .expect_err("ninth member must be rejected");
        assert_eq!(error, RosterError::GroupFull { max_members: 8 });
    }

    #[test]
    fn invite_expires_or_hits_usage_limit() {
        let group_id = GroupId::new("rimus_group").expect("group id should be valid");
        let mut invite = InviteToken::ephemeral(
            group_id,
            "ROOM-2026",
            Duration::from_secs(30),
            1,
            InviteTransport::QrCode,
        )
        .expect("invite should be valid");

        let now = invite.issued_at + Duration::from_secs(5);
        assert!(invite.is_valid_at(now));
        invite.consume(now).expect("first usage should work");

        let error = invite
            .consume(now)
            .expect_err("second usage must be rejected");
        assert_eq!(error, InviteError::UsageLimitReached);
    }

    #[test]
    fn attachment_limit_is_enforced() {
        let policy = GroupPolicy::mvp();
        let message = ChatMessage {
            message_id: MessageId::new("message_1").expect("message id should be valid"),
            author_id: MemberId::new("owner").expect("member id should be valid"),
            kind: MessageKind::Attachment,
            text: "photo dump".to_string(),
            reply_to: None,
            attachments: vec![AttachmentMeta {
                attachment_id: "att-1".to_string(),
                file_name: "clip.mov".to_string(),
                kind: AttachmentKind::Video,
                size_bytes: u64::from(policy.max_attachment_size_mb + 1) * 1024 * 1024,
            }],
            reactions: Vec::new(),
            created_at: SystemTime::UNIX_EPOCH,
        };

        assert!(message.validate_against(&policy).is_err());
    }

    fn sample_member(member_id: &str, display_name: &str, device_id: &str) -> MemberProfile {
        let device = DeviceProfile::new(
            DeviceId::new(device_id).expect("device id should be valid"),
            format!("{display_name} laptop"),
            Platform::Windows,
            VerificationState::VerifiedViaQr,
        )
        .expect("device should be valid");

        MemberProfile::new(
            MemberId::new(member_id).expect("member id should be valid"),
            display_name,
            vec![device],
            PresenceState::LanOnline,
            "4000 5000 6000 7000",
        )
        .expect("member should be valid")
    }
}
