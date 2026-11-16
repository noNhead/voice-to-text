//! LocalAgreement-2: превращает меняющиеся гипотезы batch-модели в стабильный текст.
//!
//! Слово подтверждается, когда оно совпало в двух последовательных гипотезах для одного и того же
//! растущего окна. Подтверждённое не меняется никогда, остаток — «нестабильный хвост».
//! Идея из `whisper_streaming` (Macháček et al., 2023), реализация своя.

use vtt_plugin_api::Word;

use crate::text::norm_token;

#[must_use]
pub fn join_words(words: &[Word]) -> String {
    words
        .iter()
        .map(|w| w.text.as_str())
        .collect::<String>()
        .trim()
        .to_owned()
}

#[derive(Debug, Clone, PartialEq)]
pub struct StableText {
    /// Всё подтверждённое в текущей фразе.
    pub committed: Vec<Word>,
    /// Текущий хвост, может измениться.
    pub unstable: Vec<Word>,
}

impl StableText {
    #[must_use]
    pub fn committed_text(&self) -> String {
        join_words(&self.committed)
    }

    /// Хвост с ведущим пробелом, если перед ним есть подтверждённый текст.
    #[must_use]
    pub fn unstable_text(&self) -> String {
        let text: String = self.unstable.iter().map(|w| w.text.as_str()).collect();
        if self.committed.is_empty() {
            text.trim().to_owned()
        } else {
            text
        }
    }
}

/// Работает в пределах одной фразы (одного окна сегментатора). [`Self::finalize`] закрывает фразу.
#[derive(Debug, Default)]
pub struct LocalAgreementStabilizer {
    committed: Vec<Word>,
    prev_tail: Vec<Word>,
}

impl LocalAgreementStabilizer {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `words` — полная гипотеза для текущего окна (тайминги от начала окна).
    pub fn update(&mut self, words: &[Word]) -> StableText {
        let mut tail = self.strip_committed(words);
        let agreed = self
            .prev_tail
            .iter()
            .zip(&tail)
            .take_while(|(a, b)| {
                let a = norm_token(&a.text);
                !a.is_empty() && a == norm_token(&b.text)
            })
            .count();
        self.committed.extend(tail.drain(..agreed));
        self.prev_tail.clone_from(&tail);
        StableText {
            committed: self.committed.clone(),
            unstable: tail,
        }
    }

    fn strip_committed(&self, words: &[Word]) -> Vec<Word> {
        let n = self.committed.len();
        let Some(last) = self.committed.last() else {
            return words.to_vec();
        };
        let same_head = words.len() >= n
            && words[..n]
                .iter()
                .zip(&self.committed)
                .all(|(a, b)| norm_token(&a.text) == norm_token(&b.text));
        if same_head {
            return words[n..].to_vec();
        }
        // Гипотеза разошлась с подтверждённым (модель переписала начало).
        // Подтверждённое не меняем, продолжаем с позиции по времени.
        words
            .iter()
            .filter(|w| w.t_start >= last.t_end - 0.05)
            .cloned()
            .collect()
    }

    /// Закрывает фразу и возвращает её целиком. `words` — гипотеза для полного окна.
    pub fn finalize(&mut self, words: Option<&[Word]>) -> Vec<Word> {
        let tail = match words {
            Some(w) => self.strip_committed(w),
            None => std::mem::take(&mut self.prev_tail),
        };
        let mut result = std::mem::take(&mut self.committed);
        result.extend(tail);
        self.reset();
        result
    }

    pub fn reset(&mut self) {
        self.committed.clear();
        self.prev_tail.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<Word> {
        let mut t = 0.0_f32;
        text.split_whitespace()
            .enumerate()
            .map(|(i, w)| {
                let word = Word {
                    text: if i == 0 {
                        w.to_owned()
                    } else {
                        format!(" {w}")
                    },
                    t_start: t,
                    t_end: t + 0.3,
                    prob: 0.9,
                };
                t += 0.35;
                word
            })
            .collect()
    }

    #[test]
    fn first_hypothesis_is_unstable() {
        let st = LocalAgreementStabilizer::new().update(&words("привет сегодня"));
        assert!(st.committed.is_empty());
        assert_eq!(st.unstable_text(), "привет сегодня");
    }

    #[test]
    fn agreement_commits_common_prefix() {
        let mut s = LocalAgreementStabilizer::new();
        s.update(&words("привет сегодня мы"));
        let st = s.update(&words("привет сегодня мы посмотрим"));
        assert_eq!(st.committed_text(), "привет сегодня мы");
        assert_eq!(st.unstable_text(), " посмотрим");
    }

    #[test]
    fn committed_never_changes() {
        let mut s = LocalAgreementStabilizer::new();
        s.update(&words("hello world"));
        s.update(&words("hello world today"));
        let st = s.update(&words("hallo world today we")); // модель переписала начало
        assert!(st.committed_text().starts_with("hello world"));
    }

    #[test]
    fn punctuation_and_case_do_not_block_agreement() {
        let mut s = LocalAgreementStabilizer::new();
        s.update(&words("Hello, world"));
        let st = s.update(&words("hello world!"));
        assert_eq!(st.committed.len(), 2);
    }

    #[test]
    fn finalize_returns_whole_phrase_and_resets() {
        let mut s = LocalAgreementStabilizer::new();
        s.update(&words("one two"));
        s.update(&words("one two three"));
        let fin = s.finalize(Some(&words("one two three four")));
        assert_eq!(join_words(&fin), "one two three four");
        assert!(s.update(&words("next")).committed.is_empty());
    }

    #[test]
    fn finalize_without_last_hypothesis_uses_tail() {
        let mut s = LocalAgreementStabilizer::new();
        s.update(&words("a b"));
        s.update(&words("a b c"));
        assert_eq!(join_words(&s.finalize(None)), "a b c");
    }
}
