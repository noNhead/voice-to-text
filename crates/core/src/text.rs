/// Слово для сравнения гипотез: только буквы и цифры в нижнем регистре.
#[must_use]
pub fn norm_token(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Фраза для сравнения: слова из букв и цифр в нижнем регистре через один пробел.
#[must_use]
pub fn norm_phrase(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for word in s
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
    {
        if !out.is_empty() {
            out.push(' ');
        }
        out.extend(word.chars().flat_map(char::to_lowercase));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes() {
        assert_eq!(norm_token(" Hello,"), "hello");
        assert_eq!(
            norm_phrase("  Продолжение   следует... "),
            "продолжение следует"
        );
        assert_eq!(norm_phrase("A.Семкин"), "a семкин");
    }
}
