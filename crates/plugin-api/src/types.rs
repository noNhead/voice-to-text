//! Типы данных API плагинов. Изменения в пределах мажорной версии — только аддитивные.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

use jiff::Zoned;
use serde::{Deserialize, Serialize, Serializer};

// ------------------------------------------------------------------ версии API

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ApiVersion {
    pub major: u16,
    pub minor: u16,
}

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

impl FromStr for ApiVersion {
    type Err = ApiRangeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || ApiRangeError(s.to_owned());
        let (major, minor) = s.split_once('.').unwrap_or((s, "0"));
        Ok(Self {
            major: major.parse().map_err(|_| err())?,
            minor: minor.parse().map_err(|_| err())?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Ge,
    Le,
    Eq,
    Gt,
    Lt,
}

/// Диапазон совместимых версий API, например `">=1.0, <2"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiRange {
    constraints: Vec<(Op, ApiVersion)>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("bad api constraint: {0:?}")]
pub struct ApiRangeError(pub String);

impl ApiRange {
    #[must_use]
    pub fn matches(&self, v: ApiVersion) -> bool {
        self.constraints.iter().all(|&(op, bound)| match op {
            Op::Ge => v >= bound,
            Op::Le => v <= bound,
            Op::Eq => v == bound,
            Op::Gt => v > bound,
            Op::Lt => v < bound,
        })
    }
}

impl FromStr for ApiRange {
    type Err = ApiRangeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut constraints = Vec::new();
        for part in s.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            let (op, rest) = [
                (">=", Op::Ge),
                ("<=", Op::Le),
                ("==", Op::Eq),
                (">", Op::Gt),
                ("<", Op::Lt),
            ]
            .iter()
            .find_map(|(prefix, op)| part.strip_prefix(prefix).map(|r| (*op, r)))
            .ok_or_else(|| ApiRangeError(part.to_owned()))?;
            let version = rest
                .trim()
                .parse()
                .map_err(|_| ApiRangeError(part.to_owned()))?;
            constraints.push((op, version));
        }
        if constraints.is_empty() {
            return Err(ApiRangeError(s.to_owned()));
        }
        Ok(Self { constraints })
    }
}

// ------------------------------------------------------------------ манифест и модели

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum PluginKind {
    Recognizer,
    Translator,
    Output,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Device {
    Cpu,
    Cuda,
    Vulkan,
    Metal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QualityTier {
    Eco,
    Balanced,
    Quality,
    Max,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Languages {
    /// Многоязычная модель.
    Any,
    /// Только перечисленные языки (ISO 639-1).
    Only(BTreeSet<String>),
}

/// Один файл модели. Ядро скачивает его и сверяет SHA-256.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelFile {
    /// Относительный путь внутри папки модели, например `model.bin`.
    pub path: String,
    /// Только HTTPS и домены из allowlist ядра.
    pub url: String,
    /// 64 hex-символа в нижнем регистре.
    pub sha256: String,
    pub size: u64,
}

/// Модель, которую плагин умеет запускать.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelSpec {
    pub id: String,
    pub files: Vec<ModelFile>,
    /// SPDX-идентификатор лицензии весов модели.
    pub license: String,
    pub commercial_use: bool,
    /// Текст для «О программе» и NOTICE (обязателен для CC-BY).
    pub attribution: Option<String>,
    pub languages: Languages,
    pub min_ram_mb: u32,
    /// `None` — модель работает на CPU.
    pub min_vram_mb: Option<u32>,
    pub quality_tier: QualityTier,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginManifest {
    /// `^[a-z0-9][a-z0-9_.-]{1,63}$`
    pub id: String,
    pub kind: PluginKind,
    pub name: String,
    pub version: String,
    /// Совместимые версии API, например `">=1.0, <2"`.
    pub api: String,
    /// SPDX-лицензия кода плагина.
    pub license: String,
    /// Делает ли плагин сетевые запросы сам (кроме скачивания моделей ядром).
    pub network: bool,
    pub devices: BTreeSet<Device>,
    pub models: Vec<ModelSpec>,
    /// JSON Schema настроек плагина: из неё строится UI и проверяется конфиг.
    pub config_schema: serde_json::Value,
    /// Дополнительные возможности, например `{"translate_to": ["en"]}` у Whisper.
    pub capabilities: BTreeMap<String, serde_json::Value>,
}

// ------------------------------------------------------------------ распознавание

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Word {
    /// Как отдаёт модель, включая ведущий пробел, если он есть.
    pub text: String,
    /// Секунды от начала переданного окна.
    pub t_start: f32,
    pub t_end: f32,
    pub prob: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hypothesis {
    pub words: Vec<Word>,
    pub lang: Option<String>,
    pub no_speech_prob: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SegmentState {
    Partial,
    Final,
}

/// Результат streaming-модели (Vosk, Zipformer...).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamResult {
    pub state: SegmentState,
    pub words: Vec<Word>,
    pub lang: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct TranscribeOptions {
    /// `None` — автоопределение языка.
    pub lang: Option<String>,
    /// Хвост предыдущего текста для контекста.
    pub prompt: Option<String>,
    /// Перевести на английский средствами самой модели (если она это умеет).
    pub translate_to_english: bool,
}

// ------------------------------------------------------------------ перевод

#[derive(Debug, Clone, Copy)]
pub struct TranslationRequest<'a> {
    pub text: &'a str,
    pub src: &'a str,
    pub dst: &'a str,
    /// 1–3 предыдущих финальных сегмента того же источника.
    pub context: &'a [String],
    pub glossary: &'a BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LanguagePairs {
    Any,
    Only(BTreeSet<(String, String)>),
}

// ------------------------------------------------------------------ события для output-плагинов

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TranslationResult {
    pub text: Option<String>,
    /// id переводчика или цепочка `"a+b"` при пивоте.
    pub by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SegmentEvent {
    /// Стабилен между partial, final и complete.
    pub id: String,
    pub state: SegmentState,
    /// Все переводы готовы (или истёк таймаут).
    pub complete: bool,
    pub source: String,
    pub lang_src: Option<String>,
    pub text_src: String,
    pub text_src_unstable: String,
    pub translations: BTreeMap<String, TranslationResult>,
    /// В таймзоне пользователя, сериализуется как RFC 3339 с миллисекундами и смещением.
    #[serde(serialize_with = "ser_rfc3339")]
    pub t_start: Zoned,
    #[serde(serialize_with = "ser_rfc3339")]
    pub t_end: Zoned,
    pub confidence: Option<f32>,
    pub recognizer: String,
}

fn ser_rfc3339<S: Serializer>(z: &Zoned, s: S) -> Result<S::Ok, S::Error> {
    s.collect_str(&z.strftime("%Y-%m-%dT%H:%M:%S%.3f%:z"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ComponentState {
    Starting,
    Loading,
    Running,
    Degraded,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StatusEvent {
    /// `source:mic`, `plugin:whisper-cpp`, ...
    pub component: String,
    pub state: ComponentState,
    pub detail: String,
    /// 0..1 при загрузке моделей.
    pub progress: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClearEvent {
    /// `None` — очистить все источники.
    pub source: Option<String>,
}

/// Событие шины. JSON-представление — публичный контракт для оверлея и интеграций.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
#[non_exhaustive]
pub enum Event {
    Segment(SegmentEvent),
    Status(StatusEvent),
    Clear(ClearEvent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginStatusState {
    Ok,
    Degraded,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginStatus {
    pub state: PluginStatusState,
    pub detail: String,
}
