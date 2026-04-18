use std::fmt;

use crate::futhark;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pronunciation {
    pub word: String,
    pub ipa: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Futharkation {
    pub word: String,
    pub letters: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingIpaMapping {
    pub word: String,
    pub ipa: String,
    pub symbol: char,
}

impl fmt::Display for MissingIpaMapping {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Missing futhark mapping for IPA symbol '{}' in word '{}' ({})",
            self.symbol, self.word, self.ipa
        )
    }
}

impl Pronunciation {
    pub fn to_futharkation(&self) -> Result<Futharkation, MissingIpaMapping> {
        let mut letters = String::new();

        for symbol in self.ipa.chars() {
            if should_skip_ipa_symbol(symbol) {
                continue;
            }

            let Some(letter) = naive_ipa_to_futhark(symbol) else {
                return Err(MissingIpaMapping {
                    word: self.word.clone(),
                    ipa: self.ipa.clone(),
                    symbol,
                });
            };

            debug_assert!(futhark::letter_to_index(letter).is_some());
            letters.push(letter);
        }

        Ok(Futharkation {
            word: self.word.clone(),
            letters,
        })
    }
}

pub fn load_default_pronunciations() -> Result<Vec<Pronunciation>, String> {
    parse_pronunciations(include_str!("../assets/en_US.txt"))
}

pub fn parse_pronunciations(source: &str) -> Result<Vec<Pronunciation>, String> {
    let mut items = Vec::new();

    for (line_number, raw_line) in source.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        let pronunciation = parse_pronunciation_line(line)
            .map_err(|message| format!("line {}: {}", line_number + 1, message))?;

        items.push(pronunciation);
    }

    Ok(items)
}

fn parse_pronunciation_line(line: &str) -> Result<Pronunciation, String> {
    let Some((word_part, after_slash)) = line.split_once('/') else {
        return Err(format!("missing IPA start slash: {line}"));
    };

    let Some((ipa_part, _)) = after_slash.split_once('/') else {
        return Err(format!("missing IPA end slash: {line}"));
    };

    let word = word_part.trim();
    if word.is_empty() {
        return Err(format!("missing word before IPA: {line}"));
    }

    let ipa = ipa_part.trim();
    if ipa.is_empty() {
        return Err(format!("missing IPA body: {line}"));
    }

    Ok(Pronunciation {
        word: word.to_string(),
        ipa: ipa.to_string(),
    })
}

fn should_skip_ipa_symbol(symbol: char) -> bool {
    matches!(
        symbol,
        'ˈ' | 'ˌ' | '.' | ' ' | 'ː' | '(' | ')' | ',' | '-' | '"'
    )
}

fn naive_ipa_to_futhark(symbol: char) -> Option<char> {
    match symbol {
        'p' => Some('p'),
        'b' => Some('b'),
        't' => Some('t'),
        'd' => Some('d'),
        'k' => Some('k'),
        'g' => Some('g'),
        'ɡ' => Some('g'),
        'f' => Some('f'),
        'v' => Some('f'),
        'θ' => Some('t'),
        'ð' => Some('d'),
        's' => Some('s'),
        'z' => Some('z'),
        'ʃ' => Some('s'),
        'ʒ' => Some('z'),
        'h' => Some('h'),
        'm' => Some('m'),
        'n' => Some('n'),
        'ŋ' => Some('N'),
        'l' => Some('l'),
        'ɫ' => Some('l'),
        'ɹ' => Some('r'),
        'r' => Some('r'),
        'ɾ' => Some('d'),
        'j' => Some('j'),
        'w' => Some('w'),
        'i' => Some('i'),
        'ɪ' => Some('i'),
        'e' => Some('e'),
        'ɛ' => Some('e'),
        'æ' => Some('A'),
        'a' => Some('a'),
        'ɑ' => Some('a'),
        'ɐ' => Some('a'),
        'ʌ' => Some('a'),
        'ə' => Some('e'),
        'ɚ' => Some('r'),
        'ɝ' => Some('r'),
        'u' => Some('u'),
        'ʊ' => Some('u'),
        'o' => Some('o'),
        'ɔ' => Some('o'),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_dictionary_line() {
        let parsed = parse_pronunciation_line("screamer        /ˈskɹimɝ/").expect("valid line");

        assert_eq!(
            parsed,
            Pronunciation {
                word: "screamer".to_string(),
                ipa: "ˈskɹimɝ".to_string(),
            }
        );
    }

    #[test]
    fn maps_pronunciation_to_futhark_letters() {
        let pronunciation = Pronunciation {
            word: "test".to_string(),
            ipa: "ˈtɛst".to_string(),
        };

        let mapped = pronunciation.to_futharkation().expect("mapped");
        assert_eq!(mapped.letters, "test");
    }

    #[test]
    fn dictionary_can_be_mapped_to_futhark_or_reports_missing_symbol() {
        let pronunciations = load_default_pronunciations().expect("dictionary parses");

        for pronunciation in pronunciations {
            if let Err(missing) = pronunciation.to_futharkation() {
                panic!(
                    "Missing futhark mapping for IPA symbol '{}' while mapping '{}' ({})",
                    missing.symbol, missing.word, missing.ipa
                );
            }
        }
    }
}
