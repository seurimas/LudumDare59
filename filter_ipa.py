#!/usr/bin/env python3
import re
import sys
import unicodedata

# Ensure UTF-8 output
if sys.stdout.encoding != 'utf-8':
    sys.stdout.reconfigure(encoding='utf-8')

# Read the en_US.txt file
with open('assets/en_US.txt', 'r', encoding='utf-8') as f:
    lines = f.readlines()

# Known IPA digraphs/affricates that should count as single phonemes
IPA_DIGRAPHS = {'tʃ', 'dʒ', 'ʃ', 'ʒ', 'θ', 'ð', 'ŋ', 'ʤ', 'ʧ'}

def count_ipa_phonemes(ipa_str):
    """
    Count actual phonemes in an IPA string.
    Handles combining marks, affricates, and Unicode normalization.
    """
    # Normalize to NFD (decomposed form) to separate base chars from combining marks
    ipa_nfd = unicodedata.normalize('NFD', ipa_str)
    
    # Remove combining marks, but keep base characters
    # Combining marks are in the Mn (Mark, Nonspacing) category
    ipa_clean = ''.join(c for c in ipa_nfd if unicodedata.category(c) != 'Mn')
    
    # Remove spaces and syllable markers
    ipa_clean = re.sub(r'[\s\.\|ˌˈ‖]', '', ipa_clean)
    
    # Count characters - each remaining char is a phoneme
    return len(ipa_clean), ipa_clean

# Filter for 5-IPA words
five_ipa_words = []

for line in lines:
    if '\t' not in line:
        continue
    
    parts = line.strip().split('\t')
    if len(parts) < 2:
        continue
    
    word = parts[0]
    ipa = parts[1]
    
    # Remove the slashes
    ipa_clean = ipa.strip('/').strip()
    
    # Count phonemes
    phoneme_count, ipa_normalized = count_ipa_phonemes(ipa_clean)
    
    if phoneme_count == 5:
        five_ipa_words.append((word, ipa, ipa_normalized, phoneme_count))

# Sort by word for easier scanning
five_ipa_words.sort(key=lambda x: x[0])

# Print results
for word, ipa, clean, count in five_ipa_words:
    print(word)
