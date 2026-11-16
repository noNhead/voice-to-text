//! Потребление ресурсов текущим процессом — для замеров и бенчмарков.

use std::time::Duration;

use crate::error::AudioError;
use crate::win;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessUsage {
    /// Суммарное время CPU (user + kernel) с запуска процесса. Шаг ~15.6 мс.
    pub cpu_time: Duration,
    /// Такты процессора с запуска процесса — для точных замеров малых нагрузок.
    pub cpu_cycles: u64,
    pub working_set_bytes: u64,
    pub peak_working_set_bytes: u64,
}

impl ProcessUsage {
    pub fn current() -> Result<Self, AudioError> {
        let (working_set_bytes, peak_working_set_bytes) = win::process_memory()?;
        Ok(Self {
            cpu_time: win::process_cpu_time()?,
            cpu_cycles: win::process_cycles()?,
            working_set_bytes,
            peak_working_set_bytes,
        })
    }
}
