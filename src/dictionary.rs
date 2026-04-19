use std::fmt;

use crate::futhark;
use rand::Rng;
use rand::seq::IteratorRandom;

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
        let mut chars = self.ipa.chars().peekable();

        while let Some(symbol) = chars.next() {
            if should_skip_ipa_symbol(symbol) {
                continue;
            }

            if let Some(letter) = ipa_digraph_to_futhark(symbol, chars.peek().copied()) {
                chars.next();
                debug_assert!(futhark::letter_to_index(letter).is_some());
                letters.push(letter);
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

pub fn load_default_futharkations() -> Result<Vec<Futharkation>, String> {
    let pronunciations = load_default_pronunciations()?;
    collect_futharkations(&pronunciations)
}

pub fn futharkation_from_word(word: &str) -> Result<Futharkation, String> {
    let pronunciations = load_default_pronunciations()?;
    futharkation_from_word_in_pronunciations(&pronunciations, word)
}

pub fn futharkation_from_word_with_override(
    word: &str,
    override_letters: Option<&str>,
) -> Result<Futharkation, String> {
    if let Some(letters) = override_letters {
        let letters = letters.trim();
        if letters.is_empty() {
            return Err(format!("spell '{word}' has an empty futharkation override"));
        }

        if let Some(invalid) = letters
            .chars()
            .find(|&symbol| futhark::letter_to_index(symbol).is_none())
        {
            return Err(format!(
                "spell '{word}' has invalid futhark symbol '{invalid}' in override '{letters}'"
            ));
        }

        return Ok(Futharkation {
            word: word.to_string(),
            letters: letters.to_string(),
        });
    }

    futharkation_from_word(word)
}

pub fn random_futharkation_with_rune_length<R: Rng + ?Sized>(
    rune_length: usize,
    rng: &mut R,
) -> Result<Futharkation, String> {
    let pronunciations = load_default_pronunciations()?;
    random_futharkation_with_rune_length_from_pronunciations(&pronunciations, rune_length, rng)
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

fn collect_futharkations(pronunciations: &[Pronunciation]) -> Result<Vec<Futharkation>, String> {
    pronunciations
        .iter()
        .map(|pronunciation| {
            pronunciation
                .to_futharkation()
                .map_err(|missing| missing.to_string())
        })
        .collect()
}

fn futharkation_from_word_in_pronunciations(
    pronunciations: &[Pronunciation],
    word: &str,
) -> Result<Futharkation, String> {
    let Some(pronunciation) = pronunciations
        .iter()
        .find(|pronunciation| pronunciation.word.eq_ignore_ascii_case(word))
    else {
        return Err(format!(
            "word '{word}' was not found in default pronunciations"
        ));
    };

    let mut mapped = pronunciation
        .to_futharkation()
        .map_err(|missing| missing.to_string())?;
    mapped.word = word.to_string();
    Ok(mapped)
}

fn random_futharkation_with_rune_length_from_pronunciations<R: Rng + ?Sized>(
    pronunciations: &[Pronunciation],
    rune_length: usize,
    rng: &mut R,
) -> Result<Futharkation, String> {
    let matches: Vec<Futharkation> = collect_futharkations(pronunciations)?
        .into_iter()
        .filter(|futharkation| futharkation.letters.chars().count() == rune_length)
        .collect();

    matches.into_iter().choose(rng).ok_or_else(|| {
        format!("no futharkation found with rune length {rune_length} in the default dictionary")
    })
}

fn ipa_digraph_to_futhark(symbol: char, next: Option<char>) -> Option<char> {
    match (symbol, next?) {
        ('d', 'ʒ') => Some('j'),
        _ => None,
    }
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
        'θ' => Some('T'),
        'ð' => Some('T'),
        'ʃ' => Some('S'),
        'ʒ' => Some('S'),
        's' => Some('s'),
        'z' => Some('z'),
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
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use std::collections::BTreeSet;

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
    fn maps_dz_affricate_to_j() {
        let pronunciation = Pronunciation {
            word: "judge".to_string(),
            ipa: "ˈdʒədʒ".to_string(),
        };

        let mapped = pronunciation.to_futharkation().expect("mapped");
        assert_eq!(mapped.letters, "jej");
    }

    #[test]
    fn dictionary_reports_no_missing_ipa_symbols() {
        let pronunciations = load_default_pronunciations().expect("dictionary parses");
        let mut missing_symbols = BTreeSet::new();

        for pronunciation in pronunciations {
            if let Err(missing) = pronunciation.to_futharkation() {
                missing_symbols.insert(missing.symbol);
            }
        }

        assert_eq!(missing_symbols.into_iter().collect::<Vec<char>>(), vec![]);
    }

    #[test]
    fn random_futharkation_filters_by_rune_length() {
        let pronunciations = vec![
            Pronunciation {
                word: "short".to_string(),
                ipa: "fu".to_string(),
            },
            Pronunciation {
                word: "exact".to_string(),
                ipa: "futar".to_string(),
            },
            Pronunciation {
                word: "other".to_string(),
                ipa: "fut".to_string(),
            },
        ];
        let mut rng = StdRng::seed_from_u64(7);

        let selected =
            random_futharkation_with_rune_length_from_pronunciations(&pronunciations, 5, &mut rng)
                .expect("a five-rune entry");

        assert_eq!(selected.word, "exact");
        assert_eq!(selected.letters.chars().count(), 5);
    }

    #[test]
    fn finds_futharkation_by_word_case_insensitive() {
        let pronunciations = vec![Pronunciation {
            word: "icebolt".to_string(),
            ipa: "isbolt".to_string(),
        }];

        let mapped =
            futharkation_from_word_in_pronunciations(&pronunciations, "IceBolt").expect("mapped");

        assert_eq!(mapped.word, "IceBolt");
        assert_eq!(mapped.letters, "isbolt");
    }

    #[test]
    fn override_futharkation_validates_letters() {
        let mapped = futharkation_from_word_with_override("icebolt", Some("iSbolt"))
            .expect("override accepted");
        assert_eq!(mapped.letters, "iSbolt");

        let error = futharkation_from_word_with_override("icebolt", Some("bad!"))
            .expect_err("invalid override rejected");
        assert!(error.contains("invalid futhark symbol"));
    }

    #[test]
    fn missing_dictionary_word_requires_override() {
        let error = futharkation_from_word("icebolt").expect_err("word should be missing");
        assert!(error.contains("not found in default pronunciations"));

        let mapped = futharkation_from_word_with_override("icebolt", Some("isebalt"))
            .expect("override accepted");
        assert_eq!(mapped.word, "icebolt");
        assert_eq!(mapped.letters, "isebalt");
    }
}
