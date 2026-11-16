//! Тонкие обёртки над `WinAPI`. Единственное место крейта с `unsafe`.
#![allow(unsafe_code)]

use std::time::Duration;

use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE};
use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows::Win32::System::Threading::{
    AvRevertMmThreadCharacteristics, AvSetMmThreadCharacteristicsW, GetCurrentProcess,
    GetProcessTimes, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    QueryFullProcessImageNameW,
};
use windows::core::{PWSTR, w};

use crate::error::{AudioError, Context};

/// Поток зарегистрирован в MMCSS как «Audio», пока жив guard.
#[derive(Debug)]
pub(crate) struct MmcssGuard(HANDLE);

pub(crate) fn join_mmcss_audio() -> Result<MmcssGuard, AudioError> {
    let mut task_index = 0u32;
    // SAFETY: имя задачи — статическая NUL-терминированная строка (`w!`), `task_index` —
    // валидный указатель на локальную переменную на время вызова.
    let handle = unsafe { AvSetMmThreadCharacteristicsW(w!("Audio"), &raw mut task_index) }
        .ctx("AvSetMmThreadCharacteristicsW")?;
    Ok(MmcssGuard(handle))
}

impl Drop for MmcssGuard {
    fn drop(&mut self) {
        // SAFETY: хэндл получен от AvSetMmThreadCharacteristicsW на этом же потоке
        // (guard не Send) и освобождается ровно один раз.
        let _ = unsafe { AvRevertMmThreadCharacteristics(self.0) };
    }
}

/// Закрывает хэндл процесса при выходе из области видимости.
struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: хэндл получен от OpenProcess, принадлежит нам и закрывается ровно один раз.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

/// Имя исполняемого файла процесса (`Discord.exe`) или `None`, если процесс недоступен.
pub(crate) fn process_image_name(pid: u32) -> Option<String> {
    // SAFETY: OpenProcess не требует инвариантов от вызывающего; результат проверяется.
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
    let handle = OwnedHandle(handle);
    let mut buf = [0u16; 1024];
    let mut len = u32::try_from(buf.len()).ok()?;
    // SAFETY: `buf` — локальный массив на `len` элементов, хэндл открыт с нужными правами;
    // функция пишет не больше `len` символов и возвращает фактическую длину в `len`.
    unsafe {
        QueryFullProcessImageNameW(
            handle.0,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &raw mut len,
        )
    }
    .ok()?;
    let path = String::from_utf16_lossy(buf.get(..len as usize)?);
    path.rsplit(['\\', '/']).next().map(str::to_owned)
}

fn filetime_to_duration(ft: FILETIME) -> Duration {
    let ticks = (u64::from(ft.dwHighDateTime) << 32) | u64::from(ft.dwLowDateTime);
    Duration::from_nanos(ticks.saturating_mul(100))
}

/// Суммарное время CPU (user + kernel) текущего процесса.
pub(crate) fn process_cpu_time() -> Result<Duration, AudioError> {
    let (mut created, mut exited, mut kernel, mut user) = Default::default();
    // SAFETY: GetCurrentProcess возвращает псевдо-хэндл, который не нужно закрывать;
    // все указатели — на локальные FILETIME.
    unsafe {
        GetProcessTimes(
            GetCurrentProcess(),
            &raw mut created,
            &raw mut exited,
            &raw mut kernel,
            &raw mut user,
        )
    }
    .ctx("GetProcessTimes")?;
    Ok(filetime_to_duration(kernel) + filetime_to_duration(user))
}

/// (рабочий набор, пиковый рабочий набор) текущего процесса в байтах.
pub(crate) fn process_memory() -> Result<(u64, u64), AudioError> {
    let mut counters = PROCESS_MEMORY_COUNTERS::default();
    let size = u32::try_from(size_of::<PROCESS_MEMORY_COUNTERS>()).unwrap_or(u32::MAX);
    // SAFETY: псевдо-хэндл текущего процесса; `counters` — локальная структура размера `size`.
    unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &raw mut counters, size) }
        .ctx("GetProcessMemoryInfo")?;
    Ok((
        counters.WorkingSetSize as u64,
        counters.PeakWorkingSetSize as u64,
    ))
}

/// Такты процессора, потраченные текущим процессом (точнее, чем `GetProcessTimes`, у которого
/// шаг ~15.6 мс).
pub(crate) fn process_cycles() -> Result<u64, AudioError> {
    let mut cycles = 0u64;
    // SAFETY: псевдо-хэндл текущего процесса; `cycles` — локальная переменная.
    unsafe {
        windows::Win32::System::WindowsProgramming::QueryProcessCycleTime(
            GetCurrentProcess(),
            &raw mut cycles,
        )
    }
    .ctx("QueryProcessCycleTime")?;
    Ok(cycles)
}
