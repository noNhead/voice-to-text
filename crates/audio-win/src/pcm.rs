//! Перекладка сэмплов в кольцевой буфер без выделения памяти.

#![cfg_attr(not(windows), allow(dead_code))]

use rtrb::Producer;

/// Результат записи: сколько сэмплов записано и сколько потеряно (буфер полон).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Pushed {
    pub(crate) written: usize,
    pub(crate) dropped: usize,
}

/// Переписывает f32 little-endian сэмплы из байтов WASAPI. Неполный хвост (< 4 байт) игнорируется.
pub(crate) fn push_f32le(bytes: &[u8], producer: &mut Producer<f32>) -> Pushed {
    let total = bytes.len() / 4;
    let samples = bytes
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    push_iter(total, samples, producer)
}

/// Записывает `n` нулевых сэмплов.
pub(crate) fn push_silence(n: usize, producer: &mut Producer<f32>) -> Pushed {
    push_iter(n, std::iter::repeat_n(0.0, n), producer)
}

fn push_iter(
    total: usize,
    samples: impl Iterator<Item = f32>,
    producer: &mut Producer<f32>,
) -> Pushed {
    let free = producer.slots().min(total);
    let written = match producer.write_chunk_uninit(free) {
        Ok(chunk) if free > 0 => chunk.fill_from_iter(samples),
        _ => 0,
    };
    Pushed {
        written,
        dropped: total - written,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtrb::RingBuffer;

    fn bytes(samples: &[f32]) -> Vec<u8> {
        samples.iter().flat_map(|s| s.to_le_bytes()).collect()
    }

    fn drain(c: &mut rtrb::Consumer<f32>) -> Vec<f32> {
        let mut out = Vec::new();
        while let Ok(s) = c.pop() {
            out.push(s);
        }
        out
    }

    #[test]
    fn converts_f32le() {
        let (mut p, mut c) = RingBuffer::new(16);
        let r = push_f32le(&bytes(&[0.5, -0.25, 1.0]), &mut p);
        assert_eq!(
            r,
            Pushed {
                written: 3,
                dropped: 0
            }
        );
        assert_eq!(drain(&mut c), [0.5, -0.25, 1.0]);
    }

    #[test]
    fn counts_dropped_when_full() {
        let (mut p, mut c) = RingBuffer::new(2);
        let r = push_f32le(&bytes(&[0.1, 0.2, 0.3, 0.4]), &mut p);
        assert_eq!(
            r,
            Pushed {
                written: 2,
                dropped: 2
            }
        );
        assert_eq!(drain(&mut c), [0.1, 0.2]);
        let r = push_f32le(&bytes(&[0.5]), &mut p);
        assert_eq!(
            r,
            Pushed {
                written: 1,
                dropped: 0
            }
        );
    }

    #[test]
    fn full_buffer_writes_nothing() {
        let (mut p, _c) = RingBuffer::new(1);
        push_silence(1, &mut p);
        assert_eq!(
            push_silence(5, &mut p),
            Pushed {
                written: 0,
                dropped: 5
            }
        );
    }

    #[test]
    fn ignores_partial_trailing_bytes() {
        let (mut p, mut c) = RingBuffer::new(8);
        let mut b = bytes(&[0.75]);
        b.extend_from_slice(&[1, 2]);
        assert_eq!(push_f32le(&b, &mut p).written, 1);
        assert_eq!(drain(&mut c), [0.75]);
    }

    #[test]
    fn silence_is_zero() {
        let (mut p, mut c) = RingBuffer::new(8);
        assert_eq!(push_silence(3, &mut p).written, 3);
        assert_eq!(drain(&mut c), [0.0, 0.0, 0.0]);
    }
}
