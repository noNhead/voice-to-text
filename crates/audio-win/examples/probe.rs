//! Фаза 0: проверка захвата звука и замер ресурсов.
//!
//! ```text
//! cargo run -p vtt-audio-win --release --example probe -- list
//! cargo run -p vtt-audio-win --release --example probe -- capture mic --seconds 10 --out mic.wav
//! cargo run -p vtt-audio-win --release --example probe -- capture loopback --seconds 10
//! cargo run -p vtt-audio-win --release --example probe -- capture app:Discord --seconds 30
//! cargo run -p vtt-audio-win --release --example probe -- capture pid:1234
//! ```
//!
//! Источники: `mic`, `mic:<часть имени>`, `loopback`, `loopback:<часть имени>`, `app:<exe>`, `pid:<N>`.
//! Каждую секунду печатает: записано/потеряно/дополнено тишиной, пакеты, разрывы, уровень (dBFS),
//! CPU процесса (% от всей машины) и рабочий набор. WAV пишется 16 кГц моно 16 бит.

// Утилита разработчика: печать в консоль и простые приведения типов для отчёта здесь уместны.
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

#[cfg(not(windows))]
fn main() {
    eprintln!("probe работает только в Windows");
}

#[cfg(windows)]
fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match win::run(&args) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ошибка: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(windows)]
mod win {
    use std::error::Error;
    use std::path::PathBuf;
    use std::sync::atomic::Ordering::Relaxed;
    use std::time::{Duration, Instant};

    use vtt_audio_win::{
        CaptureSource, DeviceKind, ProcessUsage, SAMPLE_RATE, find_process_by_name, list_devices,
        list_sessions, start_capture,
    };

    type Res<T> = Result<T, Box<dyn Error>>;

    pub(crate) fn run(args: &[String]) -> Res<()> {
        match args.first().map(String::as_str) {
            Some("list") => list(),
            Some("capture") => capture(&args[1..]),
            _ => Err("использование: probe list | probe capture <источник> [--seconds N] [--out file.wav]".into()),
        }
    }

    fn list() -> Res<()> {
        println!("Устройства:");
        for d in list_devices()? {
            let kind = match d.kind {
                DeviceKind::Input => "ввод ",
                DeviceKind::Output => "вывод",
            };
            let default = if d.is_default { "*" } else { " " };
            println!("  {default} {kind}  {}", d.name);
        }
        println!("Аудиосессии приложений:");
        for s in list_sessions()? {
            let state = if s.active {
                "играет"
            } else {
                "тихо  "
            };
            println!(
                "  pid {:>6}  {state}  {:<24}  {}",
                s.pid,
                s.process_name.as_deref().unwrap_or("?"),
                s.device_name
            );
        }
        Ok(())
    }

    fn find_device(kind: DeviceKind, part: &str) -> Res<String> {
        let part = part.to_lowercase();
        list_devices()?
            .into_iter()
            .find(|d| d.kind == kind && d.name.to_lowercase().contains(&part))
            .map(|d| d.id)
            .ok_or_else(|| format!("устройство {part:?} не найдено").into())
    }

    fn parse_source(spec: &str) -> Res<CaptureSource> {
        let (kind, arg) = spec.split_once(':').unwrap_or((spec, ""));
        Ok(match (kind, arg) {
            ("mic", "") => CaptureSource::Input { device_id: None },
            ("mic", name) => CaptureSource::Input {
                device_id: Some(find_device(DeviceKind::Input, name)?),
            },
            ("loopback", "") => CaptureSource::OutputLoopback { device_id: None },
            ("loopback", name) => CaptureSource::OutputLoopback {
                device_id: Some(find_device(DeviceKind::Output, name)?),
            },
            ("pid", pid) => CaptureSource::Process {
                pid: pid.parse()?,
                include_tree: true,
            },
            ("app", name) => CaptureSource::Process {
                pid: find_process_by_name(name)?,
                include_tree: true,
            },
            _ => return Err(format!("неизвестный источник {spec:?}").into()),
        })
    }

    fn capture(args: &[String]) -> Res<()> {
        let source = parse_source(args.first().ok_or("не указан источник")?)?;
        let mut seconds = 10u64;
        let mut out: Option<PathBuf> = None;
        let mut it = args[1..].iter();
        while let Some(a) = it.next() {
            match a.as_str() {
                "--seconds" => seconds = it.next().ok_or("нет значения --seconds")?.parse()?,
                "--out" => out = Some(it.next().ok_or("нет значения --out")?.into()),
                other => return Err(format!("неизвестный аргумент {other:?}").into()),
            }
        }

        let cores = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
        // Номинальная частота для пересчёта тактов в долю ядра; задаётся через VTT_CPU_GHZ.
        let nominal_ghz: f64 = std::env::var("VTT_CPU_GHZ")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3.2);
        // 2 секунды запаса: читатель забирает данные каждые 10 мс.
        let (producer, mut consumer) = rtrb::RingBuffer::new(SAMPLE_RATE as usize * 2);
        let handle = start_capture(source.clone(), producer)?;
        println!("захват: {source:?}");
        // Замер начинаем после открытия устройства: старт процесса и инициализация COM не в счёт.
        let usage0 = ProcessUsage::current()?;
        let cycles0 = usage0.cpu_cycles;

        let mut wav = match &out {
            Some(path) => Some(hound::WavWriter::create(
                path,
                hound::WavSpec {
                    channels: 1,
                    sample_rate: SAMPLE_RATE,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                },
            )?),
            None => None,
        };

        let start = Instant::now();
        let mut next_report = start + Duration::from_secs(1);
        let mut last_usage = ProcessUsage::current()?;
        let mut last_t = start;
        let (mut sum_sq, mut count, mut peak) = (0.0f64, 0u64, 0.0f32);
        let mut cpu_samples = Vec::new();
        println!(
            "   t  записано  потеряно  тишина  пакеты  разрывы   RMS dBFS   пик dBFS   CPU %   RAM МБ"
        );
        while start.elapsed() < Duration::from_secs(seconds) {
            if !handle.is_running() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
            let n = consumer.slots();
            if n > 0 {
                let chunk = consumer.read_chunk(n)?;
                let (a, b) = chunk.as_slices();
                for &s in a.iter().chain(b) {
                    sum_sq += f64::from(s) * f64::from(s);
                    peak = peak.max(s.abs());
                    count += 1;
                    if let Some(w) = wav.as_mut() {
                        #[allow(clippy::cast_possible_truncation)] // ограничено clamp
                        w.write_sample((s.clamp(-1.0, 1.0) * 32767.0) as i16)?;
                    }
                }
                chunk.commit_all();
            }
            if Instant::now() >= next_report {
                let usage = ProcessUsage::current()?;
                let now = Instant::now();
                let cpu = usage
                    .cpu_time
                    .saturating_sub(last_usage.cpu_time)
                    .as_secs_f64()
                    / (now - last_t).as_secs_f64()
                    / cores as f64
                    * 100.0;
                cpu_samples.push(cpu);
                let st = handle.stats();
                println!(
                    "{:>4}  {:>8}  {:>8}  {:>6}  {:>6}  {:>7}  {:>9}  {:>9}  {:>6.2}  {:>7.1}",
                    start.elapsed().as_secs(),
                    st.samples.load(Relaxed),
                    st.dropped.load(Relaxed),
                    st.padded.load(Relaxed),
                    st.packets.load(Relaxed),
                    st.discontinuities.load(Relaxed),
                    db(if count > 0 {
                        (sum_sq / count as f64).sqrt()
                    } else {
                        0.0
                    }),
                    db(f64::from(peak)),
                    cpu,
                    usage.working_set_bytes as f64 / 1_048_576.0
                );
                (sum_sq, count, peak) = (0.0, 0, 0.0);
                last_usage = usage;
                last_t = now;
                next_report += Duration::from_secs(1);
            }
        }
        let elapsed = start.elapsed();
        let st = handle.stats();
        let samples = st.samples.load(Relaxed);
        let dropped = st.dropped.load(Relaxed);
        let max_packet = st.max_packet_frames.load(Relaxed);
        let mmcss = st.mmcss.load(Relaxed);
        handle.stop()?;
        if let Some(w) = wav {
            w.finalize()?;
        }
        let usage = ProcessUsage::current()?;
        let total_cpu = usage.cpu_time.saturating_sub(usage0.cpu_time).as_secs_f64();
        cpu_samples.sort_by(f64::total_cmp);
        println!("--- итог (MMCSS: {})", if mmcss { "да" } else { "нет" });
        println!(
            "время {:.2} с, сэмплов {samples} (ожидалось ≈ {}), потеряно {dropped}, макс. пакет {max_packet} кадров ({:.1} мс)",
            elapsed.as_secs_f64(),
            (elapsed.as_secs_f64() * f64::from(SAMPLE_RATE)) as u64,
            max_packet as f64 * 1000.0 / f64::from(SAMPLE_RATE)
        );
        let mcycles_per_s = (usage.cpu_cycles - cycles0) as f64 / elapsed.as_secs_f64() / 1e6;
        println!(
            "CPU: {:.2} Мтактов/с ≈ {:.4} % одного ядра на {:.1} ГГц; по GetProcessTimes {:.3} с за {:.1} с (медиана по секундам {:.3} % машины)",
            mcycles_per_s,
            mcycles_per_s / (nominal_ghz * 1000.0) * 100.0,
            nominal_ghz,
            total_cpu,
            elapsed.as_secs_f64(),
            cpu_samples
                .get(cpu_samples.len() / 2)
                .copied()
                .unwrap_or(0.0),
        );
        println!(
            "RAM: рабочий набор {:.1} МБ (пик {:.1} МБ)",
            usage.working_set_bytes as f64 / 1_048_576.0,
            usage.peak_working_set_bytes as f64 / 1_048_576.0
        );
        if let Some(path) = out {
            println!("WAV: {}", path.display());
        }
        Ok(())
    }

    fn db(x: f64) -> String {
        if x <= 1e-9 {
            "-inf".into()
        } else {
            format!("{:.1}", 20.0 * x.log10())
        }
    }
}
