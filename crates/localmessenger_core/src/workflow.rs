use crate::config::{GroupConfig, TransportMode};
use crate::domain::{
    DeviceId, DeviceProfile, GroupRoster, MemberId, MemberProfile, Platform, PresenceState,
};
use crate::security::{RekeyPlan, RekeyReason, SafetyChecklist, VerificationState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectBlueprint {
    pub config: GroupConfig,
    pub roster: GroupRoster,
    pub safety: SafetyChecklist,
    pub initial_rekey_strategy: RekeyPlan,
    pub priorities: Vec<String>,
}

pub fn mvp_blueprint() -> ProjectBlueprint {
    let owner = sample_member(
        "rimus",
        "Rimus",
        "rimus-main-win",
        Platform::Windows,
        PresenceState::LanOnline,
    );

    ProjectBlueprint {
        config: GroupConfig {
            transport_modes: vec![TransportMode::LocalLanMdns, TransportMode::BluetoothFallback],
            ..GroupConfig::demo()
        },
        roster: GroupRoster::new(owner),
        safety: SafetyChecklist::recommended(),
        initial_rekey_strategy: RekeyPlan {
            next_epoch: 2,
            reason: RekeyReason::MemberRemoved("left-member".to_string()),
            revoked_devices: vec![
                DeviceId::new("old-phone").expect("static sample device id must be valid"),
            ],
        },
        priorities: next_milestones()
            .into_iter()
            .map(str::to_string)
            .collect(),
    }
}

pub fn next_milestones() -> [&'static str; 4] {
    [
        "Wire audited cryptography instead of placeholder modeling",
        "Implement mDNS discovery and QUIC transport",
        "Persist encrypted local history and file metadata",
        "Wrap the Rust core in a Tauri 2 client for desktop and Android",
    ]
}

fn sample_member(
    member_id: &str,
    display_name: &str,
    device_id: &str,
    platform: Platform,
    presence: PresenceState,
) -> MemberProfile {
    let device = DeviceProfile::new(
        DeviceId::new(device_id).expect("static sample device id must be valid"),
        format!("{display_name} primary device"),
        platform,
        VerificationState::VerifiedViaQr,
    )
    .expect("static sample device should be valid");

    MemberProfile::new(
        MemberId::new(member_id).expect("static sample member id must be valid"),
        display_name,
        vec![device],
        presence,
        "4631 9011 7455 0092",
    )
    .expect("static sample member should be valid")
}
