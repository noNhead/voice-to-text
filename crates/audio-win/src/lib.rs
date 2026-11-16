//! Захват звука в Windows через WASAPI.
//!
//! Источники: устройство ввода (микрофон), loopback устройства вывода (весь системный звук)
//! и Process Loopback (звук конкретного приложения и его дочерних процессов).
//!
//! Звук всегда приходит как **f32, моно, 16 кГц**: формат и частоту конвертирует сам аудиодвижок
//! Windows (`AUTOCONVERTPCM | SRC_DEFAULT_QUALITY`), поэтому собственный ресемплер не нужен.
//! Сэмплы попадают в lock-free кольцевой буфер `rtrb`. Поток захвата не выделяет память
//! в рабочем цикле и зарегистрирован в MMCSS («Audio»).

mod gap;
mod pcm;

#[cfg(windows)]
mod capture;
#[cfg(windows)]
mod com;
#[cfg(windows)]
mod devices;
#[cfg(windows)]
mod error;
#[cfg(windows)]
mod metrics;
#[cfg(windows)]
mod sessions;
#[cfg(windows)]
mod win;

/// Частота, в которой отдаётся звук.
pub const SAMPLE_RATE: u32 = 16_000;

#[cfg(windows)]
pub use capture::{CaptureHandle, CaptureSource, CaptureStats, start_capture};
#[cfg(windows)]
pub use devices::{DeviceInfo, DeviceKind, list_devices};
#[cfg(windows)]
pub use error::AudioError;
#[cfg(windows)]
pub use metrics::ProcessUsage;
#[cfg(windows)]
pub use sessions::{AudioSessionInfo, find_process_by_name, list_sessions};
