pub const LETTERS: [char; 24] = [
    'f', 'u', '7', 'a', 'r', 'k', 'g', 'w', 'h', 'n', 'i', 'j', 'A', 'p', 'z', 's', 't', 'b', 'e',
    'm', 'l', 'N', 'd', 'o',
];

pub fn index_to_letter(index: usize) -> Option<char> {
    LETTERS.get(index).copied()
}

pub fn letter_to_index(letter: char) -> Option<usize> {
    LETTERS
        .iter()
        .position(|mapped_letter| *mapped_letter == letter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_to_letter_maps_all_futhark_entries() {
        for (index, letter) in LETTERS.iter().enumerate() {
            assert_eq!(index_to_letter(index), Some(*letter));
        }
    }

    #[test]
    fn letter_to_index_maps_all_futhark_entries() {
        for (index, letter) in LETTERS.iter().enumerate() {
            assert_eq!(letter_to_index(*letter), Some(index));
        }
    }

    #[test]
    fn unknown_values_are_rejected() {
        assert_eq!(index_to_letter(24), None);
        assert_eq!(letter_to_index('x'), None);
    }
}
