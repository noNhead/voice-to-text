//! Ядро конвейера voice-to-text — логика, не зависящая от ОС и конкретных моделей:
//! сегментация по VAD, стабилизация гипотез (LocalAgreement-2), фильтр галлюцинаций.

pub mod hallucination;
pub mod segmenter;
pub mod stabilizer;
mod text;

pub use text::{norm_phrase, norm_token};
