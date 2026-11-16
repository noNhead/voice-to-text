//! Публичный API плагинов voice-to-text.
//!
//! Плагин зависит только от этого крейта. Версия API — [`PLUGIN_API_VERSION`] (semver):
//! ломающие изменения возможны только при смене мажорной версии. См. `docs/PLUGINS.md`.

mod error;
mod traits;
mod types;
mod validation;

pub use error::{ManifestError, PluginError};
pub use traits::{
    BatchRecognizer, BoxFuture, DeviceSelection, EventStages, Output, OutputFactory, PluginContext,
    Recognizer, RecognizerFactory, RecognizerStream, StreamingRecognizer, Translator,
    TranslatorFactory, WebMount,
};
pub use types::{
    ApiRange, ApiRangeError, ApiVersion, ClearEvent, ComponentState, Device, Event, Hypothesis,
    LanguagePairs, Languages, ModelFile, ModelSpec, PluginKind, PluginManifest, PluginStatus,
    PluginStatusState, QualityTier, SegmentEvent, SegmentState, StatusEvent, StreamResult,
    TranscribeOptions, TranslationRequest, TranslationResult, Word,
};
pub use validation::{
    COMMERCIAL_LICENSE_ALLOWLIST, NON_COMMERCIAL_LICENSES, PLUGIN_API_VERSION, is_valid_plugin_id,
    manifest_problems, model_problems, validate_manifest,
};
