//! Сегментатор: по VAD собирает растущее окно речи для batch-распознавателей.
//!
//! Получает блоки по [`CHUNK`] сэмплов (16 кГц) и возвращает:
//! - [`SegmenterEventKind::Step`] — пора распознать окно ещё раз (каждые `step_ms` во время речи);
//! - [`SegmenterEventKind::End`] — фраза закончилась (пауза или достигнут максимум окна).
//!
//! Все буферы выделяются один раз в конструкторе. Каждое событие несёт копию окна (`Arc<[f32]>`),
//! потому что окно уходит в поток распознавания.

use std::sync::Arc;

pub const SAMPLE_RATE: u32 = 16_000;
/// 32 мс при 16 кГц — размер блока Silero VAD.
pub const CHUNK: usize = 512;

/// Детектор речи: вероятность речи в блоке из [`CHUNK`] сэмплов.
pub trait Vad {
    fn speech_prob(&mut self, chunk: &[f32; CHUNK]) -> f32;
}

impl<F: FnMut(&[f32; CHUNK]) -> f32> Vad for F {
    fn speech_prob(&mut self, chunk: &[f32; CHUNK]) -> f32 {
        self(chunk)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegmenterConfig {
    /// Вероятность речи для начала фразы.
    pub threshold: f32,
    /// Ниже этого — тишина (гистерезис).
    pub neg_threshold: f32,
    pub min_silence_ms: u32,
    /// Аудио до начала речи, которое добавляется в окно.
    pub pre_roll_ms: u32,
    pub step_ms: u32,
    pub max_window_ms: u32,
    /// Более короткие всплески считаются шумом.
    pub min_speech_ms: u32,
}

impl Default for SegmenterConfig {
    fn default() -> Self {
        Self {
            threshold: 0.5,
            neg_threshold: 0.35,
            min_silence_ms: 600,
            pre_roll_ms: 256,
            step_ms: 500,
            max_window_ms: 15_000,
            min_speech_ms: 200,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndReason {
    Silence,
    MaxWindow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmenterEventKind {
    Step,
    End(EndReason),
}

#[derive(Debug, Clone)]
pub struct SegmenterEvent {
    pub kind: SegmenterEventKind,
    pub audio: Arc<[f32]>,
    /// Начало окна в секундах от начала потока.
    pub window_start_s: f64,
}

fn ms_to_samples(ms: u32) -> usize {
    (u64::from(ms) * u64::from(SAMPLE_RATE) / 1000) as usize
}

#[derive(Debug)]
pub struct Segmenter<V> {
    vad: V,
    cfg: SegmenterConfig,
    max: usize,
    window: Vec<f32>,
    pre: Vec<f32>,
    pre_chunks: usize,
    pre_count: usize,
    step: usize,
    min_silence: usize,
    min_speech: usize,
    in_speech: bool,
    silence: usize,
    speech: usize,
    since_step: usize,
    pos: u64,
    window_start: u64,
    last_prob: f32,
}

impl<V: Vad> Segmenter<V> {
    pub fn new(vad: V, cfg: SegmenterConfig) -> Self {
        let max = ms_to_samples(cfg.max_window_ms).max(CHUNK * 2);
        let pre_chunks = (ms_to_samples(cfg.pre_roll_ms) / CHUNK).max(1);
        Self {
            vad,
            cfg,
            max,
            window: Vec::with_capacity(max),
            pre: vec![0.0; pre_chunks * CHUNK],
            pre_chunks,
            pre_count: 0,
            step: ms_to_samples(cfg.step_ms),
            min_silence: ms_to_samples(cfg.min_silence_ms),
            min_speech: ms_to_samples(cfg.min_speech_ms),
            in_speech: false,
            silence: 0,
            speech: 0,
            since_step: 0,
            pos: 0,
            window_start: 0,
            last_prob: 0.0,
        }
    }

    #[must_use]
    pub fn in_speech(&self) -> bool {
        self.in_speech
    }

    #[must_use]
    pub fn last_prob(&self) -> f32 {
        self.last_prob
    }

    pub fn push(&mut self, chunk: &[f32; CHUNK]) -> Option<SegmenterEvent> {
        let prob = self.vad.speech_prob(chunk);
        self.last_prob = prob;
        if !self.in_speech {
            if prob >= self.cfg.threshold {
                self.start();
            } else {
                let slot = self.pre_count % self.pre_chunks;
                self.pre[slot * CHUNK..(slot + 1) * CHUNK].copy_from_slice(chunk);
                self.pre_count += 1;
                self.pos += CHUNK as u64;
                return None;
            }
        }

        self.window.extend_from_slice(chunk);
        self.since_step += CHUNK;
        if prob < self.cfg.neg_threshold {
            self.silence += CHUNK;
        } else {
            self.silence = 0;
            self.speech += CHUNK;
        }

        let event = if self.silence >= self.min_silence {
            let ev = (self.speech >= self.min_speech)
                .then(|| self.emit(SegmenterEventKind::End(EndReason::Silence)));
            self.reset_after_end();
            ev
        } else if self.window.len() + CHUNK > self.max {
            let ev = self.emit(SegmenterEventKind::End(EndReason::MaxWindow));
            self.reset_after_end();
            Some(ev)
        } else if self.since_step >= self.step && self.speech >= self.min_speech {
            self.since_step = 0;
            Some(self.emit(SegmenterEventKind::Step))
        } else {
            None
        };
        self.pos += CHUNK as u64;
        event
    }

    fn start(&mut self) {
        let n = self.pre_count.min(self.pre_chunks);
        self.window.clear();
        // в pre-roll лежат блоки до текущего; кладём их в хронологическом порядке
        for i in 0..n {
            let slot = (self.pre_count - n + i) % self.pre_chunks;
            self.window
                .extend_from_slice(&self.pre[slot * CHUNK..(slot + 1) * CHUNK]);
        }
        self.window_start = self.pos - self.window.len() as u64;
        self.in_speech = true;
        self.silence = 0;
        self.speech = 0;
        self.since_step = 0;
    }

    fn emit(&self, kind: SegmenterEventKind) -> SegmenterEvent {
        SegmenterEvent {
            kind,
            audio: Arc::from(self.window.as_slice()),
            window_start_s: self.window_start as f64 / f64::from(SAMPLE_RATE),
        }
    }

    fn reset_after_end(&mut self) {
        self.in_speech = false;
        self.window.clear();
        self.pre_count = 0;
        self.silence = 0;
        self.speech = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Фейковый VAD: вероятность речи по номеру блока.
    fn scripted(probs: Vec<f32>) -> impl FnMut(&[f32; CHUNK]) -> f32 {
        let mut i = 0;
        move |_| {
            let p = probs.get(i).copied().unwrap_or(0.0);
            i += 1;
            p
        }
    }

    fn ms_to_chunks(ms: u32) -> usize {
        ms_to_samples(ms) / CHUNK
    }

    fn run(probs: &[f32], cfg: SegmenterConfig) -> Vec<SegmenterEvent> {
        let mut seg = Segmenter::new(scripted(probs.to_vec()), cfg);
        let mut out = Vec::new();
        for i in 0..probs.len() {
            #[allow(clippy::cast_possible_truncation)] // номер блока в тесте мал
            let chunk = [i as f32; CHUNK];
            if let Some(ev) = seg.push(&chunk) {
                out.push(ev);
            }
        }
        out
    }

    fn concat(parts: &[&[f32]]) -> Vec<f32> {
        parts.concat()
    }

    #[test]
    fn silence_emits_nothing() {
        assert!(run(&[0.0; 200], SegmenterConfig::default()).is_empty());
    }

    #[test]
    fn phrase_produces_steps_then_end() {
        let cfg = SegmenterConfig {
            step_ms: 500,
            min_silence_ms: 600,
            pre_roll_ms: 256,
            ..Default::default()
        };
        let probs = concat(&[
            &[0.0; 20],
            &vec![0.9; ms_to_chunks(2000)],
            &vec![0.0; ms_to_chunks(800)],
        ]);
        let events = run(&probs, cfg);
        let last = events.last().unwrap();
        assert_eq!(last.kind, SegmenterEventKind::End(EndReason::Silence));
        let steps = events
            .iter()
            .filter(|e| e.kind == SegmenterEventKind::Step)
            .count();
        let ends = events
            .iter()
            .filter(|e| matches!(e.kind, SegmenterEventKind::End(_)))
            .count();
        assert!(steps >= 3, "steps = {steps}");
        assert_eq!(ends, 1);
        // окно начинается с pre-roll: 8 блоков по 32 мс до начала речи
        let first = &events[0];
        let pre = ms_to_chunks(256);
        #[allow(clippy::cast_precision_loss, clippy::float_cmp)]
        {
            assert_eq!(first.audio[0], (20 - pre) as f32);
            assert_eq!(first.window_start_s, ((20 - pre) * CHUNK) as f64 / 16_000.0);
        }
    }

    #[test]
    fn short_noise_burst_is_ignored() {
        let cfg = SegmenterConfig {
            min_speech_ms: 200,
            ..Default::default()
        };
        let probs = concat(&[&[0.0; 10], &[0.9; 3], &[0.0; 40]]); // ~96 мс «речи»
        assert!(run(&probs, cfg).is_empty());
    }

    #[test]
    fn hysteresis_keeps_phrase_alive() {
        // 0.4 — между neg_threshold и threshold: это не тишина
        let probs = concat(&[&[0.9; 20], &[0.4; 40], &[0.9; 20], &[0.0; 30]]);
        let ends = run(&probs, SegmenterConfig::default())
            .iter()
            .filter(|e| matches!(e.kind, SegmenterEventKind::End(_)))
            .count();
        assert_eq!(ends, 1);
    }

    #[test]
    fn max_window_forces_end() {
        let cfg = SegmenterConfig {
            max_window_ms: 2000,
            ..Default::default()
        };
        let events = run(&vec![0.9; ms_to_chunks(5000)], cfg);
        let ends: Vec<_> = events
            .iter()
            .filter(|e| matches!(e.kind, SegmenterEventKind::End(_)))
            .collect();
        assert!(!ends.is_empty());
        assert_eq!(ends[0].kind, SegmenterEventKind::End(EndReason::MaxWindow));
        assert!(events.iter().all(|e| e.audio.len() <= 2 * 16_000));
    }

    #[test]
    fn window_never_reallocates() {
        let cfg = SegmenterConfig {
            max_window_ms: 2000,
            ..Default::default()
        };
        let mut seg = Segmenter::new(|_: &[f32; CHUNK]| 0.9, cfg);
        let cap = seg.window.capacity();
        for _ in 0..ms_to_chunks(10_000) {
            let _ = seg.push(&[0.0; CHUNK]);
            assert_eq!(seg.window.capacity(), cap);
        }
    }
}
