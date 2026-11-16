//! Перечисление аудиоустройств.

use wasapi::{Device, DeviceEnumerator, DeviceState, Direction};

use crate::com::ComGuard;
use crate::error::{AudioError, Context};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceKind {
    /// Устройство ввода (микрофон, линейный вход).
    Input,
    /// Устройство вывода: с него снимается loopback.
    Output,
}

impl DeviceKind {
    fn direction(self) -> Direction {
        match self {
            Self::Input => Direction::Capture,
            Self::Output => Direction::Render,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    /// Стабильный идентификатор устройства Windows.
    pub id: String,
    pub name: String,
    pub kind: DeviceKind,
    pub is_default: bool,
}

/// Активные устройства ввода и вывода.
pub fn list_devices() -> Result<Vec<DeviceInfo>, AudioError> {
    let _com = ComGuard::init_mta();
    let enumerator = DeviceEnumerator::new().ctx("DeviceEnumerator::new")?;
    let mut out = Vec::new();
    for kind in [DeviceKind::Input, DeviceKind::Output] {
        let default_id = enumerator
            .get_default_device(kind.direction())
            .and_then(|d| d.get_id())
            .ok();
        let collection = enumerator
            .get_device_collection(kind.direction())
            .ctx("get_device_collection")?;
        for device in &collection {
            let device = device.ctx("device collection item")?;
            if !matches!(device.get_state(), Ok(DeviceState::Active)) {
                continue;
            }
            let id = device.get_id().ctx("Device::get_id")?;
            out.push(DeviceInfo {
                name: device.get_friendlyname().unwrap_or_else(|_| id.clone()),
                is_default: default_id.as_deref() == Some(id.as_str()),
                id,
                kind,
            });
        }
    }
    Ok(out)
}

/// Устройство по id или устройство по умолчанию. COM должен быть инициализирован вызывающим.
pub(crate) fn open_device(kind: DeviceKind, id: Option<&str>) -> Result<Device, AudioError> {
    let enumerator = DeviceEnumerator::new().ctx("DeviceEnumerator::new")?;
    match id {
        None => enumerator
            .get_default_device(kind.direction())
            .ctx("get_default_device"),
        Some(id) => enumerator
            .get_device(id)
            .map_err(|_| AudioError::DeviceNotFound(id.to_owned())),
    }
}
