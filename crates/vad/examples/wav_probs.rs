//! Вероятности речи по блокам 32 мс для WAV (16 кГц, моно, 16 бит) и скорость вывода.
//!
//! ```text
//! cargo run -p vtt-vad --release --example wav_probs -- models/silero/silero_vad_v6.onnx speech.wav [probs.txt]
//! ```

// Утилита разработчика: печать в консоль и простые приведения для отчёта здесь уместны.
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::cast_precision_loss
)]

use std::fmt::Write as _;
use std::time::Instant;

use vtt_core::segmenter::CHUNK;
use vtt_vad::SileroVad;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [model, wav, rest @ ..] = args.as_slice() else {
        return Err("использование: wav_probs <model.onnx> <file.wav> [probs.txt]".into());
    };
    let mut reader = hound::WavReader::open(wav)?;
    let spec = reader.spec();
    if spec.sample_rate != 16_000 || spec.channels != 1 || spec.bits_per_sample != 16 {
        return Err(format!("нужен WAV 16 кГц моно 16 бит, а это {spec:?}").into());
    }
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.map(|v| f32::from(v) / 32768.0))
        .collect::<Result<_, _>>()?;

    let t_load = Instant::now();
    let mut vad = SileroVad::load(model.as_ref())?;
    let load = t_load.elapsed();

    let mut probs = Vec::with_capacity(samples.len() / CHUNK);
    let t0 = Instant::now();
    for chunk in samples.chunks_exact(CHUNK) {
        let chunk: &[f32; CHUNK] = chunk.try_into()?;
        probs.push(vad.try_prob(chunk)?);
    }
    let elapsed = t0.elapsed();
    let per_chunk_us = elapsed.as_secs_f64() * 1e6 / probs.len() as f64;
    let speech = probs.iter().filter(|&&p| p >= 0.5).count();
    println!(
        "блоков {}, речь в {} ({:.0} %), загрузка {:.0} мс, {:.1} мкс на блок 32 мс ≈ {:.3} % одного ядра",
        probs.len(),
        speech,
        speech as f64 * 100.0 / probs.len() as f64,
        load.as_secs_f64() * 1000.0,
        per_chunk_us,
        per_chunk_us / 32_000.0 * 100.0
    );
    if let Some(path) = rest.first() {
        let mut out = String::new();
        for p in &probs {
            let _ = writeln!(out, "{p:.6}");
        }
        std::fs::write(path, out)?;
        println!("вероятности: {path}");
    }
    Ok(())
}
