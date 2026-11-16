//! Трейты трёх гнёзд конвейера и контекст, который ядро даёт плагину. См. `docs/PLUGINS.md`.
//!
//! Каждое гнездо состоит из двух частей:
//! - *фабрика* (`*Factory`) — регистрируется в реестре, отдаёт манифест и создаёт экземпляр;
//! - *экземпляр* — загруженная модель или запущенный вывод.

use std::future::Future;
use std::ops::BitOr;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;

use jiff::Zoned;

use crate::error::PluginError;
use crate::types::{
    Device, Event, Hypothesis, LanguagePairs, PluginManifest, PluginStatus, StreamResult,
    TranscribeOptions, TranslationRequest,
};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceSelection {
    pub device: Device,
    /// Номер устройства (GPU), для CPU — 0.
    pub index: u32,
}

/// Что ядро даёт плагину.
pub trait PluginContext: Send + Sync {
    fn device(&self) -> DeviceSelection;
    /// Бюджет потоков инференса. Плагин обязан его соблюдать.
    fn cpu_threads(&self) -> usize;
    /// Личная папка плагина.
    fn data_dir(&self) -> &Path;
    /// Папка уже скачанной и проверенной модели.
    fn model_path(&self, model_id: &str) -> Result<PathBuf, PluginError>;
    fn report_status(&self, status: PluginStatus);
    /// Текущее время в таймзоне пользователя (подменяется в тестах).
    fn now(&self) -> Zoned;
}

// ------------------------------------------------------------------ Recognizer

/// Batch-модель (Whisper): ядро подаёт растущее окно и само стабилизирует гипотезы.
pub trait BatchRecognizer: Send {
    /// Синхронно, в потоке `asr-*`. Тайминги слов — от начала `pcm16k` (f32, моно, 16 кГц).
    fn transcribe(
        &mut self,
        pcm16k: &[f32],
        opts: &TranscribeOptions,
    ) -> Result<Hypothesis, PluginError>;
}

pub trait RecognizerStream: Send {
    fn feed(&mut self, pcm16k: &[f32]) -> Result<Vec<StreamResult>, PluginError>;
    /// Конец речи по VAD.
    fn flush(&mut self) -> Result<Vec<StreamResult>, PluginError>;
}

/// Streaming-модель (Vosk, Zipformer): сама отдаёт partial/final.
pub trait StreamingRecognizer: Send {
    fn open_stream(&mut self, lang: Option<&str>)
    -> Result<Box<dyn RecognizerStream>, PluginError>;
}

pub enum Recognizer {
    Batch(Box<dyn BatchRecognizer>),
    Streaming(Box<dyn StreamingRecognizer>),
}

impl std::fmt::Debug for Recognizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Batch(_) => "Recognizer::Batch",
            Self::Streaming(_) => "Recognizer::Streaming",
        })
    }
}

pub trait RecognizerFactory: Send + Sync {
    fn manifest(&self) -> &PluginManifest;
    fn load(
        &self,
        model_id: &str,
        ctx: Arc<dyn PluginContext>,
        config: &serde_json::Value,
    ) -> Result<Recognizer, PluginError>;
}

// ------------------------------------------------------------------ Translator

pub trait Translator: Send {
    fn pairs(&self) -> LanguagePairs;
    /// Синхронно, в потоке `mt-*`. Длина результата равна длине `batch`.
    fn translate(&mut self, batch: &[TranslationRequest<'_>]) -> Result<Vec<String>, PluginError>;
}

pub trait TranslatorFactory: Send + Sync {
    fn manifest(&self) -> &PluginManifest;
    fn load(
        &self,
        model_id: &str,
        ctx: Arc<dyn PluginContext>,
        config: &serde_json::Value,
    ) -> Result<Box<dyn Translator>, PluginError>;
}

// ------------------------------------------------------------------ Output

/// На какие стадии событий подписан output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventStages(u8);

impl EventStages {
    pub const PARTIAL: Self = Self(1);
    pub const FINAL: Self = Self(1 << 1);
    pub const COMPLETE: Self = Self(1 << 2);
    pub const STATUS: Self = Self(1 << 3);
    pub const CLEAR: Self = Self(1 << 4);
    pub const ALL: Self = Self(0b1_1111);

    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for EventStages {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Веб-часть output-плагина под `/o/<output-id>/`. Проверки Host/Origin/токенов делает ядро.
pub trait WebMount: Send {
    /// Отдать встроенный файл по относительному пути (`index.html`, `themes/default.css`).
    fn add_static(&mut self, path: &str, content_type: &'static str, body: &'static [u8]);
}

pub trait Output: Send {
    fn accepts(&self) -> EventStages;
    fn start<'a>(
        &'a mut self,
        web: Option<&'a mut dyn WebMount>,
    ) -> BoxFuture<'a, Result<(), PluginError>>;
    fn handle<'a>(&'a mut self, event: &'a Event) -> BoxFuture<'a, Result<(), PluginError>>;
    fn stop(&mut self) -> BoxFuture<'_, Result<(), PluginError>>;
}

pub trait OutputFactory: Send + Sync {
    fn manifest(&self) -> &PluginManifest;
    fn create(
        &self,
        ctx: Arc<dyn PluginContext>,
        config: &serde_json::Value,
    ) -> Result<Box<dyn Output>, PluginError>;
}

#[cfg(test)]
mod tests {
    use super::EventStages;

    #[test]
    fn event_stages_combine() {
        let s = EventStages::FINAL | EventStages::COMPLETE;
        assert!(s.contains(EventStages::FINAL));
        assert!(!s.contains(EventStages::PARTIAL));
        assert!(EventStages::ALL.contains(s));
    }
}
