import random

with open('five_ipa_words_fixed.txt', 'r', encoding='utf-8') as f:
    words = [line.strip() for line in f if line.strip()]

print('\n'.join(random.sample(words, 5)))
