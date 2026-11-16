//! Заполнение пауз тишиной.
//!
//! WASAPI loopback (системный звук и Process Loopback) **не присылает пакеты, пока ничего не
//! играет**. Без заполнения время потока «останавливается», и сегментатор не видит паузу после
//! фразы, пока снова не зазвучит звук. Поэтому, если пакетов нет дольше порога, вставляем столько
//! тишины, сколько реально прошло времени.
//!
//! Дополняем только по факту пропуска (а не сверяя с часами весь поток): так расхождение часов
//! устройства и системы не накапливается и не создаёт ложных вставок посреди речи.

#![cfg_attr(not(windows), allow(dead_code))]

use std::time::{Duration, Instant};

#[derive(Debug)]
pub(crate) struct GapFiller {
    last: Instant,
    threshold: Duration,
    sample_rate: u32,
    /// Дробная часть кадра, перенесённая на следующий раз (чтобы округление не накапливалось).
    carry_ns: u128,
}

impl GapFiller {
    pub(crate) fn new(now: Instant, threshold: Duration, sample_rate: u32) -> Self {
        Self {
            last: now,
            threshold,
            sample_rate,
            carry_ns: 0,
        }
    }

    /// Пришли настоящие данные.
    pub(crate) fn on_data(&mut self, now: Instant) {
        self.last = now;
        self.carry_ns = 0;
    }

    /// Поток проснулся без данных. Возвращает, сколько кадров тишины вставить.
    pub(crate) fn on_idle(&mut self, now: Instant) -> usize {
        let gap = now.saturating_duration_since(self.last);
        if gap < self.threshold {
            return 0;
        }
        self.last = now;
        let total = gap.as_nanos() * u128::from(self.sample_rate) + self.carry_ns;
        let frames = total / 1_000_000_000;
        self.carry_ns = total % 1_000_000_000;
        usize::try_from(frames).unwrap_or(usize::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: Duration = Duration::from_millis(1);

    #[test]
    fn no_padding_while_data_flows() {
        let t0 = Instant::now();
        let mut g = GapFiller::new(t0, 100 * MS, 16_000);
        for i in 1..50 {
            g.on_data(t0 + i * 10 * MS);
            assert_eq!(g.on_idle(t0 + i * 10 * MS + 5 * MS), 0);
        }
    }

    #[test]
    fn pads_whole_gap_after_threshold() {
        let t0 = Instant::now();
        let mut g = GapFiller::new(t0, 100 * MS, 16_000);
        assert_eq!(g.on_idle(t0 + 50 * MS), 0);
        // 120 мс без данных → 1920 кадров тишины
        assert_eq!(g.on_idle(t0 + 120 * MS), 1920);
        // дальше считаем от момента вставки
        assert_eq!(g.on_idle(t0 + 150 * MS), 0);
        assert_eq!(g.on_idle(t0 + 250 * MS), 2080);
    }

    #[test]
    fn long_silence_keeps_exact_time() {
        let t0 = Instant::now();
        let mut g = GapFiller::new(t0, 100 * MS, 16_000);
        let mut total = 0;
        // 10 секунд пробуждений по 20 мс без данных
        for i in 1..=500 {
            total += g.on_idle(t0 + i * 20 * MS);
        }
        assert_eq!(total, 160_000);
    }

    #[test]
    fn data_resets_gap() {
        let t0 = Instant::now();
        let mut g = GapFiller::new(t0, 100 * MS, 16_000);
        g.on_data(t0 + 90 * MS);
        assert_eq!(g.on_idle(t0 + 150 * MS), 0);
    }
}
