//! Фаза 0: живой звук → Silero VAD → сегментатор. Печатает границы фраз и потребление ресурсов.
//! Звук и текст никуда не сохраняются.
//!
//! ```text
//! cargo run -p vtt-vad --release --example live -- models/silero/silero_vad_v6.onnx mic 30
//! cargo run -p vtt-vad --release --example live -- models/silero/silero_vad_v6.onnx app:chrome 30
//! ```
//! Источник: `mic`, `loopback`, `app:<exe>`, `pid:<N>`; последний аргумент — длительность в секундах.

// Утилита разработчика: печать в консоль и простые приведения для отчёта здесь уместны.
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::cast_precision_loss,
    clippy::too_many_lines
)]

#[cfg(not(windows))]
fn main() {
    eprintln!("live работает только в Windows");
}

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::atomic::Ordering::Relaxed;
    use std::time::{Duration, Instant};

    use vtt_audio_win::{
        CaptureSource, ProcessUsage, SAMPLE_RATE, find_process_by_name, start_capture,
    };
    use vtt_core::segmenter::{CHUNK, EndReason, Segmenter, SegmenterConfig, SegmenterEventKind};
    use vtt_vad::SileroVad;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let [model, source, rest @ ..] = args.as_slice() else {
        return Err(
            "использование: live <model.onnx> <mic|loopback|app:EXE|pid:N> [секунды]".into(),
        );
    };
    let seconds: u64 = rest.first().map_or(Ok(30), |s| s.parse())?;
    let source = match source.split_once(':') {
        None if source == "mic" => CaptureSource::Input { device_id: None },
        None if source == "loopback" => CaptureSource::OutputLoopback { device_id: None },
        Some(("app", name)) => CaptureSource::Process {
            pid: find_process_by_name(name)?,
            include_tree: true,
        },
        Some(("pid", pid)) => CaptureSource::Process {
            pid: pid.parse()?,
            include_tree: true,
        },
        _ => return Err(format!("неизвестный источник {source:?}").into()),
    };
    let ghz: f64 = std::env::var("VTT_CPU_GHZ")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3.2);

    let vad = SileroVad::load(model.as_ref())?;
    let mut seg = Segmenter::new(vad, SegmenterConfig::default());
    let (producer, mut consumer) = rtrb::RingBuffer::new(SAMPLE_RATE as usize * 2);
    let capture = start_capture(source.clone(), producer)?;
    println!("слушаю {source:?} {seconds} с");

    let usage0 = ProcessUsage::current()?;
    let start = Instant::now();
    let mut chunk = [0.0f32; CHUNK];
    let mut stream_samples = 0u64;
    let mut was_speech = false;
    let (mut phrases, mut speech_s, mut steps) = (0u32, 0.0f64, 0u32);
    let t = |samples: u64| samples as f64 / f64::from(SAMPLE_RATE);
    while start.elapsed() < Duration::from_secs(seconds) {
        let Ok(read) = consumer.read_chunk(CHUNK) else {
            std::thread::sleep(Duration::from_millis(8));
            continue;
        };
        let (a, b) = read.as_slices();
        chunk[..a.len()].copy_from_slice(a);
        chunk[a.len()..].copy_from_slice(b);
        read.commit_all();
        stream_samples += CHUNK as u64;

        let event = seg.push(&chunk);
        if seg.in_speech() && !was_speech {
            println!("{:>8.2} с  ▶ начало речи", t(stream_samples));
        }
        was_speech = seg.in_speech();
        match event.map(|e| (e.kind, e.audio.len())) {
            Some((SegmenterEventKind::Step, _)) => steps += 1,
            Some((SegmenterEventKind::End(reason), len)) => {
                phrases += 1;
                speech_s += t(len as u64);
                let why = match reason {
                    EndReason::Silence => "пауза",
                    EndReason::MaxWindow => "максимум окна",
                };
                println!(
                    "{:>8.2} с  ■ конец фразы ({why}), окно {:.2} с",
                    t(stream_samples),
                    t(len as u64)
                );
            }
            None => {}
        }
    }
    let elapsed = start.elapsed().as_secs_f64();
    let usage = ProcessUsage::current()?;
    let st = capture.stats();
    let dropped = st.dropped.load(Relaxed);
    let mcycles = (usage.cpu_cycles - usage0.cpu_cycles) as f64 / elapsed / 1e6;
    drop(capture);
    println!("--- итог");
    println!(
        "фраз {phrases}, речи {speech_s:.1} с из {elapsed:.1} с, шагов распознавания {steps}, \
         потеряно сэмплов {dropped}, ошибок VAD {}",
        seg_errors(&seg)
    );
    println!(
        "CPU (захват + VAD + сегментатор): {mcycles:.1} Мтактов/с ≈ {:.2} % одного ядра на {ghz} ГГц; \
         RAM {:.1} МБ (пик {:.1} МБ)",
        mcycles / (ghz * 1000.0) * 100.0,
        usage.working_set_bytes as f64 / 1_048_576.0,
        usage.peak_working_set_bytes as f64 / 1_048_576.0
    );
    Ok(())
}

#[cfg(windows)]
fn seg_errors(seg: &vtt_core::segmenter::Segmenter<vtt_vad::SileroVad>) -> u64 {
    seg.vad().errors()
}
