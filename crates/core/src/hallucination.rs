//! Фильтр галлюцинаций распознавателя — одинаковый для всех моделей.

use std::collections::HashSet;

use crate::text::norm_phrase;

/// Фразы, которые Whisper часто «слышит» в тишине и шуме
/// (они попали в обучающие данные из субтитров).
pub const DEFAULT_PHANTOMS: &[&str] = &[
    "продолжение следует",
    "субтитры сделал dimatorzok",
    "субтитры создавал dimatorzok",
    "редактор субтитров а семкин корректор а егорова",
    "спасибо за просмотр",
    "подписывайтесь на канал",
    "thanks for watching",
    "thank you for watching",
    "thank you",
    "please subscribe",
    "subtitles by the amara org community",
    "untertitel im auftrag des zdf",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    Empty,
    NoSpeech,
    LowConfidence,
    PhantomPhrase,
    Repetition,
}

#[derive(Debug, Clone)]
pub struct HallucinationFilter {
    pub max_no_speech_prob: f32,
    pub min_avg_prob: f32,
    pub max_ngram_repeats: usize,
    phantoms: HashSet<String>,
}

impl Default for HallucinationFilter {
    fn default() -> Self {
        Self::new(DEFAULT_PHANTOMS.iter().copied())
    }
}

impl HallucinationFilter {
    pub fn new<'a>(phantoms: impl IntoIterator<Item = &'a str>) -> Self {
        Self {
            max_no_speech_prob: 0.6,
            min_avg_prob: 0.35,
            max_ngram_repeats: 3,
            phantoms: phantoms.into_iter().map(norm_phrase).collect(),
        }
    }

    /// Причина отбросить сегмент или `None`, если сегмент выглядит настоящим.
    #[must_use]
    pub fn reason(&self, text: &str, no_speech_prob: f32, avg_prob: f32) -> Option<RejectReason> {
        let norm = norm_phrase(text);
        if norm.is_empty() {
            return Some(RejectReason::Empty);
        }
        if no_speech_prob > self.max_no_speech_prob && avg_prob < 0.6 {
            return Some(RejectReason::NoSpeech);
        }
        if avg_prob < self.min_avg_prob {
            return Some(RejectReason::LowConfidence);
        }
        // Фантомная фраза может быть и настоящей («Thank you»): отбрасываем только неуверенную.
        if self.phantoms.contains(&norm) && (no_speech_prob > 0.2 || avg_prob < 0.75) {
            return Some(RejectReason::PhantomPhrase);
        }
        let tokens: Vec<&str> = norm.split(' ').collect();
        if self.repeats(&tokens) {
            return Some(RejectReason::Repetition);
        }
        None
    }

    fn repeats(&self, tokens: &[&str]) -> bool {
        for n in 1..=4 {
            if tokens.len() < n * (self.max_ngram_repeats + 1) {
                continue;
            }
            let mut run = 1;
            let mut i = n;
            while i + n <= tokens.len() {
                if tokens[i..i + n] == tokens[i - n..i] {
                    run += 1;
                    if run > self.max_ngram_repeats {
                        return true;
                    }
                } else {
                    run = 1;
                }
                i += n;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f() -> HallucinationFilter {
        HallucinationFilter::default()
    }

    #[test]
    fn normal_speech_passes() {
        assert_eq!(
            f().reason("Привет, сегодня мы посмотрим игру.", 0.05, 0.9),
            None
        );
    }

    #[test]
    fn phantoms_on_uncertain_audio() {
        for text in ["Продолжение следует...", "Thanks for watching!", "  "] {
            assert!(f().reason(text, 0.5, 0.5).is_some(), "{text:?}");
        }
    }

    #[test]
    fn confident_thank_you_is_kept() {
        assert_eq!(f().reason("Thank you.", 0.01, 0.95), None);
    }

    #[test]
    fn no_speech() {
        assert_eq!(f().reason("что-то", 0.9, 0.4), Some(RejectReason::NoSpeech));
    }

    #[test]
    fn repetition() {
        assert_eq!(
            f().reason("да да да да да да", 0.0, 0.9),
            Some(RejectReason::Repetition)
        );
        assert_eq!(
            f().reason("и вот и вот и вот и вот и вот", 0.0, 0.9),
            Some(RejectReason::Repetition)
        );
        assert_eq!(f().reason("да да да", 0.0, 0.9), None);
    }

    #[test]
    fn low_confidence() {
        assert_eq!(
            f().reason("бла", 0.0, 0.1),
            Some(RejectReason::LowConfidence)
        );
    }
}
