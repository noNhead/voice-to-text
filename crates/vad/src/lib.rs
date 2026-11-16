//! Потоковый детектор речи Silero VAD (MIT) на `tract`: чистый Rust, без нативных библиотек и DLL.
//!
//! Модель — ONNX-экспорт Silero VAD v6 с явным состоянием LSTM (входы `input[1, 576]`, `h`, `c`).
//! На каждый блок из [`CHUNK`] сэмплов (32 мс при 16 кГц) подаются 64 сэмпла контекста из конца
//! предыдущего блока + сам блок, а состояние LSTM переносится между вызовами.
//!
//! Один экземпляр — один источник звука: состояние не делится между потоками.

use std::path::Path;

use tract_onnx::prelude::*;
use vtt_core::segmenter::{CHUNK, Vad};

/// Сколько сэмплов предыдущего блока подаётся как контекст.
const CONTEXT: usize = 64;
const INPUT: usize = CONTEXT + CHUNK;
const STATE: usize = 128;

/// SHA-256 модели, с которой проверены точность и скорость (см. `docs/BENCHMARKS.md`).
pub const SILERO_V6_SHA256: &str =
    "4cbf549b8326f60f80f2536d9eefeb450a9abe83365a098031c89719f1be17d2";

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum VadError {
    #[error("failed to load VAD model: {0}")]
    Load(String),
    #[error("VAD inference failed: {0}")]
    Inference(String),
}

type Model = TypedRunnableModel;

pub struct SileroVad {
    model: Arc<Model>,
    /// Состояние плана создаётся один раз: `plan.run()` пересоздавал бы его (и память под граф) на каждый блок.
    state: TypedSimpleState,
    /// `[1, 576]`: 64 сэмпла контекста + 512 сэмплов блока.
    input: Tensor,
    h: Tensor,
    c: Tensor,
    errors: u64,
}

impl std::fmt::Debug for SileroVad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SileroVad")
            .field("errors", &self.errors)
            .finish_non_exhaustive()
    }
}

impl SileroVad {
    /// Загружает и оптимизирует модель под фиксированные формы входов.
    pub fn load(path: &Path) -> Result<Self, VadError> {
        let load = || -> TractResult<Arc<Model>> {
            tract_onnx::onnx()
                .model_for_path(path)?
                .with_input_fact(0, f32::fact([1, INPUT]).into())?
                .with_input_fact(1, f32::fact([1, 1, STATE]).into())?
                .with_input_fact(2, f32::fact([1, 1, STATE]).into())?
                .into_optimized()?
                .into_runnable()
        };
        let model = load().map_err(|e| VadError::Load(format!("{}: {e:#}", path.display())))?;
        Self::with_model(model)
    }

    /// Новый поток с тем же графом модели: граф загружается один раз на процесс.
    pub fn fork(&self) -> Result<Self, VadError> {
        Self::with_model(Arc::clone(&self.model))
    }

    fn with_model(model: Arc<Model>) -> Result<Self, VadError> {
        let load_err = |e: TractError| VadError::Load(format!("{e:#}"));
        let zeros = |shape: &[usize]| Tensor::zero::<f32>(shape).map_err(load_err);
        Ok(Self {
            state: model.spawn().map_err(load_err)?,
            model,
            input: zeros(&[1, INPUT])?,
            h: zeros(&[1, 1, STATE])?,
            c: zeros(&[1, 1, STATE])?,
            errors: 0,
        })
    }

    /// Сбросить состояние (новый поток или после разрыва звука).
    pub fn reset(&mut self) {
        for t in [&mut self.input, &mut self.h, &mut self.c] {
            if let Ok(mut v) = t.try_as_plain_ram_mut() {
                v.as_slice_mut::<f32>().map(|s| s.fill(0.0)).ok();
            }
        }
    }

    /// Сколько блоков не удалось обработать (для них возвращена вероятность 0).
    #[must_use]
    pub fn errors(&self) -> u64 {
        self.errors
    }

    /// Вероятность речи в блоке. При ошибке вывода состояние LSTM не меняется.
    pub fn try_prob(&mut self, chunk: &[f32; CHUNK]) -> Result<f32, VadError> {
        let err = |e: TractError| VadError::Inference(format!("{e:#}"));
        {
            let mut view = self.input.try_as_plain_ram_mut().map_err(err)?;
            let input = view.as_slice_mut::<f32>().map_err(err)?;
            // контекст: последние 64 сэмпла предыдущего блока
            input.copy_within(INPUT - CONTEXT.., 0);
            input[CONTEXT..].copy_from_slice(chunk);
        }
        let inputs = tvec!(
            self.input.clone().into_tvalue(),
            self.h.clone().into_tvalue(),
            self.c.clone().into_tvalue()
        );
        let mut out = self.state.run(inputs).map_err(err)?.into_iter();
        let (Some(prob), Some(h), Some(c), None) = (out.next(), out.next(), out.next(), out.next())
        else {
            return Err(VadError::Inference(
                "expected 3 outputs (prob, h, c)".into(),
            ));
        };
        self.h = h.into_tensor();
        self.c = c.into_tensor();
        let view = prob.try_as_plain_ram().map_err(err)?;
        view.as_slice::<f32>()
            .map_err(err)?
            .first()
            .copied()
            .ok_or_else(|| VadError::Inference("empty output".into()))
    }
}

impl Vad for SileroVad {
    /// При ошибке вывода считает блок тишиной и увеличивает [`SileroVad::errors`].
    fn speech_prob(&mut self, chunk: &[f32; CHUNK]) -> f32 {
        self.try_prob(chunk).unwrap_or_else(|_| {
            self.errors += 1;
            0.0
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Детектор переезжает в поток dsp, поэтому обязан быть `Send`.
    #[test]
    fn vad_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<SileroVad>();
    }

    /// Путь к модели: `VTT_SILERO_MODEL` или `models/silero/silero_vad_v6.onnx` в корне workspace.
    fn model_path() -> std::path::PathBuf {
        std::env::var_os("VTT_SILERO_MODEL").map_or_else(
            || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../models/silero/silero_vad_v6.onnx"),
            Into::into,
        )
    }

    /// Детерминированный «шум» без зависимостей.
    fn noise(seed: u32, amp: f32) -> [f32; CHUNK] {
        let mut x = seed.wrapping_mul(2_654_435_761).max(1);
        std::array::from_fn(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            #[allow(clippy::cast_precision_loss)] // u32 → f32 для генерации шума
            let v = (x as f32 / u32::MAX as f32) * 2.0 - 1.0;
            v * amp
        })
    }

    #[test]
    #[ignore = "нужна модель Silero: models/silero/silero_vad_v6.onnx или VTT_SILERO_MODEL"]
    fn silence_is_not_speech_and_reset_is_deterministic() {
        let mut vad = SileroVad::load(&model_path()).expect("model loads");
        let silence = [0.0f32; CHUNK];
        for _ in 0..30 {
            assert!(vad.try_prob(&silence).unwrap() < 0.1);
        }
        let run = |vad: &mut SileroVad| -> Vec<f32> {
            (0..20)
                .map(|i| vad.try_prob(&noise(i, 0.3)).unwrap())
                .collect()
        };
        vad.reset();
        let a = run(&mut vad);
        vad.reset();
        let b = run(&mut vad);
        assert_eq!(a, b, "после reset результат должен повторяться");
        let mut other = vad.fork().expect("fork");
        assert_eq!(run(&mut other), a, "fork начинает с чистого состояния");
        assert_eq!(vad.errors(), 0);
    }

    #[test]
    fn missing_model_is_an_error() {
        let err = SileroVad::load(Path::new("no/such/model.onnx")).unwrap_err();
        assert!(matches!(err, VadError::Load(_)));
    }
}
