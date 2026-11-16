//! Аудиосессии приложений: из них пользователь выбирает, чей звук слушать.

use wasapi::{DeviceEnumerator, DeviceState, Direction, SessionState};

use crate::com::ComGuard;
use crate::error::{AudioError, Context};
use crate::win;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioSessionInfo {
    pub pid: u32,
    /// Имя исполняемого файла (`Discord.exe`), если процесс доступен.
    pub process_name: Option<String>,
    /// Имя, которое приложение задало сессии (часто пустое).
    pub display_name: String,
    /// Сессия сейчас воспроизводит звук.
    pub active: bool,
    pub device_name: String,
}

/// Сессии на всех активных устройствах вывода. Системные звуки (pid 0) пропускаются.
pub fn list_sessions() -> Result<Vec<AudioSessionInfo>, AudioError> {
    let _com = ComGuard::init_mta();
    let enumerator = DeviceEnumerator::new().ctx("DeviceEnumerator::new")?;
    let devices = enumerator
        .get_device_collection(Direction::Render)
        .ctx("get_device_collection")?;
    let mut out = Vec::new();
    for device in &devices {
        let device = device.ctx("device collection item")?;
        if !matches!(device.get_state(), Ok(DeviceState::Active)) {
            continue;
        }
        let device_name = device.get_friendlyname().unwrap_or_default();
        let sessions = device
            .get_iaudiosessionmanager()
            .and_then(|m| m.get_audiosessionenumerator())
            .ctx("audio session enumerator")?;
        for i in 0..sessions.get_count().ctx("session count")? {
            let Ok(session) = sessions.get_session(i) else {
                continue;
            };
            let pid = session.get_process_id().unwrap_or(0);
            if pid == 0 {
                continue;
            }
            out.push(AudioSessionInfo {
                pid,
                process_name: win::process_image_name(pid),
                display_name: session.get_display_name().unwrap_or_default(),
                active: matches!(session.get_state(), Ok(SessionState::Active)),
                device_name: device_name.clone(),
            });
        }
    }
    Ok(out)
}

/// PID приложения по имени исполняемого файла (без учёта регистра, `.exe` можно не писать).
/// Если сессий несколько, предпочитается та, что сейчас играет звук.
pub fn find_process_by_name(name: &str) -> Result<u32, AudioError> {
    let wanted = name.trim().to_ascii_lowercase();
    let wanted = wanted.strip_suffix(".exe").unwrap_or(&wanted);
    let sessions = list_sessions()?;
    let matching = sessions.iter().filter(|s| {
        s.process_name.as_deref().is_some_and(|p| {
            let p = p.to_ascii_lowercase();
            p.strip_suffix(".exe").unwrap_or(&p) == wanted
        })
    });
    matching
        .max_by_key(|s| s.active)
        .map(|s| s.pid)
        .ok_or_else(|| AudioError::ProcessNotFound(name.to_owned()))
}
