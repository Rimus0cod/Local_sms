use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::device::Device;
use crate::error::CoreError;
use crate::ids::{DeviceId, MemberId};
use crate::verification::SafetyNumber;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberProfile {
    member_id: MemberId,
    display_name: String,
    devices: BTreeMap<DeviceId, Device>,
}

impl MemberProfile {
    pub fn new(member_id: MemberId, display_name: impl Into<String>) -> Result<Self, CoreError> {
        let display_name = display_name.into();
        if display_name.trim().is_empty() {
            return Err(CoreError::EmptyDisplayName);
        }

        Ok(Self {
            member_id,
            display_name,
            devices: BTreeMap::new(),
        })
    }

    pub fn member_id(&self) -> &MemberId {
        &self.member_id
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn devices(&self) -> impl Iterator<Item = &Device> {
        self.devices.values()
    }

    pub fn device(&self, device_id: &DeviceId) -> Option<&Device> {
        self.devices.get(device_id)
    }

    pub fn device_mut(&mut self, device_id: &DeviceId) -> Option<&mut Device> {
        self.devices.get_mut(device_id)
    }

    pub fn add_device(&mut self, device: Device) -> Result<(), CoreError> {
        if device.owner_member_id() != &self.member_id {
            return Err(CoreError::ForeignDeviceOwner {
                expected_member_id: self.member_id.to_string(),
                actual_member_id: device.owner_member_id().to_string(),
            });
        }

        if self.devices.contains_key(device.device_id()) {
            return Err(CoreError::DuplicateDevice(device.device_id().to_string()));
        }

        self.devices.insert(device.device_id().clone(), device);
        Ok(())
    }

    pub fn verified_devices(&self) -> Vec<&Device> {
        self.devices
            .values()
            .filter(|device| device.is_verified())
            .collect()
    }

    pub fn verify_device_by_qr(
        &mut self,
        device_id: &DeviceId,
        payload_bytes: &[u8],
    ) -> Result<(), CoreError> {
        let device = self
            .devices
            .get_mut(device_id)
            .ok_or_else(|| CoreError::MissingDevice(device_id.to_string()))?;
        device.verify_with_qr_payload(payload_bytes)
    }

    pub fn verify_device_by_safety_number(
        &mut self,
        device_id: &DeviceId,
        local_reference_device: &Device,
        presented_safety_number: &SafetyNumber,
    ) -> Result<(), CoreError> {
        let device = self
            .devices
            .get_mut(device_id)
            .ok_or_else(|| CoreError::MissingDevice(device_id.to_string()))?;
        device.verify_with_safety_number(local_reference_device, presented_safety_number)
    }
}
