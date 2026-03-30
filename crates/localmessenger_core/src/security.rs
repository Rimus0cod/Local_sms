use crate::domain::DeviceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CryptoProfile {
    SignalStyleSenderKeys,
    NoiseHandshakeSenderKeys,
}

impl CryptoProfile {
    pub fn label(&self) -> &'static str {
        match self {
            Self::SignalStyleSenderKeys => "Signal-style sender keys",
            Self::NoiseHandshakeSenderKeys => "Noise handshake + sender keys",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationState {
    Pending,
    VerifiedViaQr,
    VerifiedViaSafetyNumber,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RekeyReason {
    MemberRemoved(String),
    DeviceRevoked(String),
    SafetyNumberChanged(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RekeyPlan {
    pub next_epoch: u64,
    pub reason: RekeyReason,
    pub revoked_devices: Vec<DeviceId>,
}

impl RekeyPlan {
    pub fn summary(&self) -> String {
        let reason = match &self.reason {
            RekeyReason::MemberRemoved(member_id) => format!("member {member_id} removed"),
            RekeyReason::DeviceRevoked(device_id) => format!("device {device_id} revoked"),
            RekeyReason::SafetyNumberChanged(member_id) => {
                format!("safety number changed for {member_id}")
            }
        };

        format!("epoch {} rekey scheduled because {}", self.next_epoch, reason)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafetyChecklist {
    pub qr_verification_required: bool,
    pub manual_safety_number_supported: bool,
    pub forward_secrecy_required: bool,
    pub local_storage_only: bool,
    pub automatic_rekey_on_member_change: bool,
}

impl SafetyChecklist {
    pub fn recommended() -> Self {
        Self {
            qr_verification_required: true,
            manual_safety_number_supported: true,
            forward_secrecy_required: true,
            local_storage_only: true,
            automatic_rekey_on_member_change: true,
        }
    }
}
