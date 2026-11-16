//! Поток захвата: WASAPI (event-driven, shared, autoconvert) → кольцевой буфер `rtrb`.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use rtrb::Producer;
use wasapi::{AudioClient, Direction, SampleType, StreamMode, WasapiError, WaveFormat};

use crate::SAMPLE_RATE;
use crate::com::ComGuard;
use crate::devices::{DeviceKind, open_device};
use crate::error::{AudioError, Context};
use crate::gap::GapFiller;
use crate::pcm::{Pushed, push_f32le, push_silence};
use crate::win;

/// Если пакетов нет дольше этого, время потока досыпается тишиной (см. `gap.rs`).
const GAP_THRESHOLD: Duration = Duration::from_millis(100);
/// Как часто поток просыпается при отсутствии событий: проверка `stop` и заполнение пауз.
const WAKE_TIMEOUT_MS: u32 = 20;
/// Размер рабочего буфера пакета: от 100 мс до 2 с, выделяется один раз.
const MIN_PACKET_FRAMES: u32 = 1_600;
const MAX_PACKET_FRAMES: u32 = 32_000;
const BYTES_PER_FRAME: usize = 4; // f32 моно

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureSource {
    /// Устройство ввода; `None` — устройство по умолчанию.
    Input { device_id: Option<String> },
    /// Весь звук устройства вывода; `None` — устройство по умолчанию.
    OutputLoopback { device_id: Option<String> },
    /// Звук конкретного процесса (Windows 10 2004+). `include_tree` — вместе с дочерними процессами.
    Process { pid: u32, include_tree: bool },
}

/// Счётчики потока захвата. Обновляются из аудиопотока атомарно, без блокировок.
#[derive(Debug, Default)]
pub struct CaptureStats {
    /// Сэмплов записано в буфер (включая тишину-заполнение).
    pub samples: AtomicU64,
    /// Сэмплов потеряно: буфер переполнен или пакет не поместился.
    pub dropped: AtomicU64,
    /// Сэмплов тишины, вставленных в паузах loopback.
    pub padded: AtomicU64,
    pub packets: AtomicU64,
    /// Пакетов с флагом `DATA_DISCONTINUITY` (сбой непрерывности на стороне Windows).
    pub discontinuities: AtomicU64,
    pub max_packet_frames: AtomicU64,
    /// Поток захвата зарегистрирован в MMCSS («Audio»).
    pub mmcss: AtomicBool,
}

impl CaptureStats {
    fn record(&self, pushed: Pushed) {
        self.samples.fetch_add(pushed.written as u64, Relaxed);
        self.dropped.fetch_add(pushed.dropped as u64, Relaxed);
    }
}

/// Работающий захват. При drop поток останавливается.
#[derive(Debug)]
pub struct CaptureHandle {
    stop: Arc<AtomicBool>,
    stats: Arc<CaptureStats>,
    thread: Option<JoinHandle<Result<(), AudioError>>>,
}

impl CaptureHandle {
    #[must_use]
    pub fn stats(&self) -> &CaptureStats {
        &self.stats
    }

    /// Поток ещё работает (не завершился с ошибкой).
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.thread.as_ref().is_some_and(|t| !t.is_finished())
    }

    /// Останавливает захват и возвращает ошибку потока, если она была.
    pub fn stop(mut self) -> Result<(), AudioError> {
        self.shutdown()
    }

    fn shutdown(&mut self) -> Result<(), AudioError> {
        self.stop.store(true, Relaxed);
        match self.thread.take() {
            Some(t) => t.join().map_err(|_| AudioError::ThreadPanicked)?,
            None => Ok(()),
        }
    }
}

impl Drop for CaptureHandle {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

/// Запускает поток захвата. Возвращается после того, как поток открыл устройство и запустил
/// поток WASAPI (или с ошибкой инициализации).
pub fn start_capture(
    source: CaptureSource,
    producer: Producer<f32>,
) -> Result<CaptureHandle, AudioError> {
    let stop = Arc::new(AtomicBool::new(false));
    let stats = Arc::new(CaptureStats::default());
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let thread = std::thread::Builder::new()
        .name("audio-capture".into())
        .spawn({
            let (stop, stats) = (Arc::clone(&stop), Arc::clone(&stats));
            move || run(&source, producer, &stop, &stats, &ready_tx)
        })
        .map_err(AudioError::Spawn)?;
    match ready_rx.recv() {
        Ok(Ok(())) => Ok(CaptureHandle {
            stop,
            stats,
            thread: Some(thread),
        }),
        Ok(Err(e)) => {
            let _ = thread.join();
            Err(e)
        }
        Err(_) => Err(match thread.join() {
            Ok(Err(e)) => e,
            Ok(Ok(())) => AudioError::ThreadExited,
            Err(_) => AudioError::ThreadPanicked,
        }),
    }
}

fn open_client(source: &CaptureSource) -> Result<AudioClient, AudioError> {
    match source {
        CaptureSource::Input { device_id } => open_device(DeviceKind::Input, device_id.as_deref())?
            .get_iaudioclient()
            .ctx("IMMDevice::Activate(IAudioClient)"),
        CaptureSource::OutputLoopback { device_id } => {
            open_device(DeviceKind::Output, device_id.as_deref())?
                .get_iaudioclient()
                .ctx("IMMDevice::Activate(IAudioClient)")
        }
        CaptureSource::Process { pid, include_tree } => {
            AudioClient::new_application_loopback_client(*pid, *include_tree)
                .ctx("ActivateAudioInterfaceAsync(PROCESS_LOOPBACK)")
        }
    }
}

struct Stream {
    client: AudioClient,
    capture: wasapi::AudioCaptureClient,
    event: wasapi::Handle,
    packet_frames: u32,
}

fn open_stream(source: &CaptureSource) -> Result<Stream, AudioError> {
    let mut client = open_client(source)?;
    let format = WaveFormat::new(32, 32, SampleType::Float, SAMPLE_RATE as usize, 1, None);
    let mode = StreamMode::EventsShared {
        autoconvert: true,
        buffer_duration_hns: 0,
    };
    // Для устройства вывода wasapi сам добавляет AUDCLNT_STREAMFLAGS_LOOPBACK.
    client
        .initialize_client(&format, Direction::Capture, mode, None)
        .ctx("IAudioClient::Initialize")?;
    let event = client.set_get_eventhandle().ctx("SetEventHandle")?;
    let capture = client
        .get_audiocaptureclient()
        .ctx("GetService(IAudioCaptureClient)")?;
    // Для Process Loopback размер буфера бывает огромным — ограничиваем.
    let packet_frames = client
        .get_buffer_size()
        .unwrap_or(MIN_PACKET_FRAMES)
        .clamp(MIN_PACKET_FRAMES, MAX_PACKET_FRAMES);
    Ok(Stream {
        client,
        capture,
        event,
        packet_frames,
    })
}

fn run(
    source: &CaptureSource,
    mut producer: Producer<f32>,
    stop: &AtomicBool,
    stats: &CaptureStats,
    ready: &mpsc::SyncSender<Result<(), AudioError>>,
) -> Result<(), AudioError> {
    let _com = ComGuard::init_mta();
    // MMCSS не критичен: без него захват работает, просто с обычным приоритетом.
    let mmcss_guard = win::join_mmcss_audio().ok();
    stats.mmcss.store(mmcss_guard.is_some(), Relaxed);

    let stream = match open_stream(source).and_then(|s| {
        s.client.start_stream().ctx("IAudioClient::Start")?;
        Ok(s)
    }) {
        Ok(s) => s,
        Err(e) => {
            let _ = ready.send(Err(e));
            return Ok(());
        }
    };
    let _ = ready.send(Ok(()));

    // Единственное выделение памяти — до рабочего цикла.
    let mut buf = vec![0u8; stream.packet_frames as usize * BYTES_PER_FRAME];
    let mut gap = GapFiller::new(Instant::now(), GAP_THRESHOLD, SAMPLE_RATE);
    let result = capture_loop(&stream, &mut buf, &mut producer, &mut gap, stop, stats);
    let _ = stream.client.stop_stream();
    result
}

fn capture_loop(
    s: &Stream,
    buf: &mut [u8],
    producer: &mut Producer<f32>,
    gap: &mut GapFiller,
    stop: &AtomicBool,
    stats: &CaptureStats,
) -> Result<(), AudioError> {
    while !stop.load(Relaxed) && !producer.is_abandoned() {
        // Таймаут — нормальная ситуация (тишина в loopback), не ошибка.
        let _ = s.event.wait_for_event(WAKE_TIMEOUT_MS);
        let mut got_data = false;
        while s
            .capture
            .get_next_packet_size()
            .ctx("GetNextPacketSize")?
            .unwrap_or(0)
            > 0
        {
            match s.capture.read_from_device(buf) {
                Ok((0, _)) => break,
                Ok((frames, info)) => {
                    got_data = true;
                    stats.packets.fetch_add(1, Relaxed);
                    stats
                        .max_packet_frames
                        .fetch_max(u64::from(frames), Relaxed);
                    if info.flags.data_discontinuity {
                        stats.discontinuities.fetch_add(1, Relaxed);
                    }
                    let bytes = &buf[..frames as usize * BYTES_PER_FRAME];
                    stats.record(push_f32le(bytes, producer));
                }
                // Пакет больше рабочего буфера: wasapi уже освободил его, данные потеряны.
                Err(WasapiError::DataLengthTooShort { expected, .. }) => {
                    stats.dropped.fetch_add(expected as u64, Relaxed);
                }
                Err(e) => {
                    return Err(AudioError::Wasapi {
                        context: "GetBuffer",
                        source: e,
                    });
                }
            }
        }
        let now = Instant::now();
        if got_data {
            gap.on_data(now);
        } else {
            let frames = gap.on_idle(now);
            if frames > 0 {
                let pushed = push_silence(frames, producer);
                stats.padded.fetch_add(pushed.written as u64, Relaxed);
                stats.record(pushed);
            }
        }
    }
    Ok(())
}
