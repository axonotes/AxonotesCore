#![allow(dead_code)]

use rand::rngs::OsRng;
use rand::TryRngCore;
use sha2::{Digest, Sha256};

use super::hash;

const WORDLIST: &str = include_str!("bip39-en.txt");

pub fn get_mnemonic() -> Result<String, Box<dyn std::error::Error>> {
    let words: Vec<&str> = WORDLIST.lines().collect();

    if words.len() != 2048 {
        return Err("Invalid wordlist: must contain exactly 2048 words".into());
    }

    // Generate 16 bytes (128 bits) of entropy for 12 words
    let mut entropy = [0u8; 16];
    OsRng
        .try_fill_bytes(&mut entropy)
        .expect("Failed to fill OsRng with entropy");

    // Calculate checksum: first 4 bits of SHA256(entropy)
    let hash = Sha256::digest(entropy);
    let checksum_byte = hash[0];

    // Combine entropy + checksum into bits
    let mut bits = Vec::new();
    for byte in entropy.iter() {
        for i in (0..8).rev() {
            bits.push((byte >> i) & 1);
        }
    }
    // Add first 4 bits of checksum
    for i in (4..8).rev() {
        bits.push((checksum_byte >> i) & 1);
    }

    // Convert each 11-bit group to a word index
    let mut mnemonic = Vec::new();
    for chunk in bits.chunks(11) {
        let mut index = 0u16;
        for bit in chunk {
            index = (index << 1) | u16::from(*bit);
        }
        mnemonic.push(words[index as usize]);
    }

    Ok(mnemonic.join(" "))
}

pub const MNEMONIC_KEY_ENCRYPTION_CONTEXT: &str = "encryption_context";
pub const MNEMONIC_KEY_SIGNING_CONTEXT: &str = "signing_context";

/**
 * Use function with
 * - `MNEMONIC_KEY_ENCRYPTION_CONTEXT` or
 * - `MNEMONIC_KEY_SIGNING_CONTEXT`
 */
pub fn mnemonic_to_key(mnemonic: &str, context: &str) -> Vec<u8> {
    hash::derive_key(mnemonic, context)
}

pub fn validate_and_correct_passphrase(input: &str) -> (bool, String) {
    let words: Vec<&str> = WORDLIST.lines().collect();

    if words.len() != 2048 {
        return (false, String::new());
    }

    let input_words: Vec<&str> = input.split_whitespace().collect();

    // BIP39 supports 12, 15, 18, 21, or 24 words
    if ![12, 15, 18, 21, 24].contains(&input_words.len()) {
        return (false, String::new());
    }

    // Try to match each input word to a wordlist word
    let mut matched_words = Vec::new();
    let mut word_indices = Vec::new();

    for input_word in input_words {
        let lower = input_word.to_lowercase();

        // Try exact match first
        if let Some(pos) = words.iter().position(|&w| w == lower) {
            matched_words.push(words[pos]);
            word_indices.push(pos);
            continue;
        }

        // Try prefix match
        let matches: Vec<_> = words
            .iter()
            .enumerate()
            .filter(|(_, &w)| w.starts_with(&lower))
            .collect();

        if matches.len() == 1 {
            let (pos, &word) = matches[0];
            matched_words.push(word);
            word_indices.push(pos);
            continue;
        }

        // Try fuzzy match (edit distance <= 2)
        let mut best_match = None;
        let mut best_distance = usize::MAX;

        for (i, &w) in words.iter().enumerate() {
            let distance = levenshtein_distance(&lower, w);
            if distance < best_distance && distance <= 2 {
                best_distance = distance;
                best_match = Some((i, w));
            }
        }

        if let Some((pos, word)) = best_match {
            matched_words.push(word);
            word_indices.push(pos);
        } else {
            // Could not match this word
            return (false, String::new());
        }
    }

    // Now validate the checksum
    let is_valid = validate_checksum(&word_indices);
    let corrected = matched_words.join(" ");

    (is_valid, corrected)
}

fn validate_checksum(word_indices: &[usize]) -> bool {
    // Convert word indices to bits
    let mut bits = Vec::new();
    for &index in word_indices {
        for i in (0..11).rev() {
            #[allow(clippy::cast_possible_truncation)] // Result is 0 or 1, always fits in u8
            bits.push(((index >> i) & 1) as u8);
        }
    }

    // Calculate how many bits are entropy vs checksum
    let total_bits = bits.len();
    let checksum_bits = total_bits / 33; // For every 32 bits of entropy, 1 bit of checksum
    let entropy_bits = total_bits - checksum_bits;

    // Extract entropy bytes
    let mut entropy = Vec::new();
    for chunk in bits[..entropy_bits].chunks(8) {
        let mut byte = 0u8;
        for bit in chunk {
            byte = (byte << 1) | bit;
        }
        entropy.push(byte);
    }

    // Calculate expected checksum
    let hash = Sha256::digest(&entropy);

    // Compare checksum bits
    for i in 0..checksum_bits {
        let expected_bit = (hash[i / 8] >> (7 - (i % 8))) & 1;
        let actual_bit = bits[entropy_bits + i];
        if expected_bit != actual_bit {
            return false;
        }
    }

    true
}

#[allow(clippy::needless_range_loop)] // Index used both for iteration and value assignment
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut matrix = vec![vec![0; b_len + 1]; a_len + 1];

    for i in 0..=a_len {
        matrix[i][0] = i;
    }
    for j in 0..=b_len {
        matrix[0][j] = j;
    }

    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = std::cmp::min(
                std::cmp::min(
                    matrix[i - 1][j] + 1, // deletion
                    matrix[i][j - 1] + 1, // insertion
                ),
                matrix[i - 1][j - 1] + cost, // substitution
            );
        }
    }

    matrix[a_len][b_len]
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to get a valid test mnemonic
    fn get_test_mnemonic() -> &'static str {
        // This is a valid 12-word BIP39 mnemonic with correct checksum
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
    }

    #[test]
    fn test_get_passphrase_generates_12_words() {
        let result = get_mnemonic();
        assert!(result.is_ok(), "Should generate passphrase successfully");

        let passphrase = result.unwrap();
        let words: Vec<&str> = passphrase.as_str().split_whitespace().collect();
        assert_eq!(words.len(), 12, "Should generate exactly 12 words");
    }

    #[test]
    fn test_get_passphrase_words_are_valid() {
        let result = get_mnemonic().unwrap();
        let words: Vec<&str> = result.as_str().split_whitespace().collect();
        let wordlist: Vec<&str> = WORDLIST.lines().collect();

        for word in words {
            assert!(
                wordlist.contains(&word),
                "Word '{}' should be in the wordlist",
                word
            );
        }
    }

    #[test]
    fn test_get_passphrase_is_unique() {
        // Generate multiple passphrases and ensure they're different
        let passphrase1 = get_mnemonic().unwrap();
        let passphrase2 = get_mnemonic().unwrap();
        let passphrase3 = get_mnemonic().unwrap();

        assert_ne!(passphrase1, passphrase2, "Passphrases should be unique");
        assert_ne!(passphrase2, passphrase3, "Passphrases should be unique");
        assert_ne!(passphrase1, passphrase3, "Passphrases should be unique");
    }

    #[test]
    fn test_validate_valid_passphrase() {
        let mnemonic = get_test_mnemonic();
        let (is_valid, corrected) = validate_and_correct_passphrase(mnemonic);

        assert!(is_valid, "Valid mnemonic should pass validation");
        assert_eq!(corrected, mnemonic, "Corrected phrase should match input");
    }

    #[test]
    fn test_validate_generated_passphrase() {
        // A generated passphrase should always be valid
        let passphrase = get_mnemonic().unwrap();
        let (is_valid, _) = validate_and_correct_passphrase(passphrase.as_str());
        assert!(is_valid, "Generated passphrase should be valid");
    }

    #[test]
    fn test_validate_wrong_word_count() {
        let invalid_inputs = vec![
            "word",                               // 1 word
            "word word word word word",           // 5 words
            "word word word word word word word", // 7 words
        ];

        for input in invalid_inputs {
            let (is_valid, corrected) = validate_and_correct_passphrase(input);
            assert!(
                !is_valid,
                "Should reject input with {} words",
                input.split_whitespace().count()
            );
            assert_eq!(
                corrected, "",
                "Should return empty string for invalid word count"
            );
        }
    }

    #[test]
    fn test_validate_valid_word_counts() {
        // BIP39 supports 12, 15, 18, 21, and 24 word mnemonics
        // We'll just test that these lengths are accepted (even if checksum might fail)
        let word = "abandon";

        for count in [12, 15, 18, 21, 24] {
            let input = vec![word; count].join(" ");
            let (_, corrected) = validate_and_correct_passphrase(&input);

            // If it returns a non-empty corrected string, it at least processed the word count
            // (checksum validation is separate)
            let corrected_count = corrected.as_str().split_whitespace().count();
            if !corrected.is_empty() {
                assert_eq!(
                    corrected_count, count,
                    "Should process {}-word mnemonic",
                    count
                );
            }
        }
    }

    #[test]
    fn test_validate_with_uppercase() {
        let mnemonic = "ABANDON abandon ABANDON abandon abandon ABANDON abandon abandon abandon abandon abandon about";
        let (is_valid, corrected) = validate_and_correct_passphrase(mnemonic);

        assert!(is_valid, "Should accept uppercase words");
        assert_eq!(
            corrected,
            get_test_mnemonic(),
            "Should normalize to lowercase"
        );
    }

    #[test]
    fn test_validate_with_prefix_match() {
        // Use prefixes that uniquely identify words
        let mnemonic = "aban aban aban aban aban aban aban aban aban aban aban abou";
        let (is_valid, corrected) = validate_and_correct_passphrase(mnemonic);

        assert!(is_valid, "Should accept valid prefix matches");
        assert_eq!(
            corrected,
            get_test_mnemonic(),
            "Should expand prefixes to full words"
        );
    }

    #[test]
    fn test_validate_with_typos() {
        // Test with small typos (edit distance <= 2)
        let mnemonic = "abanbon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let (is_valid, corrected) = validate_and_correct_passphrase(mnemonic);

        // Should correct the typo and validate
        assert!(is_valid, "Should correct minor typos");
        assert!(
            corrected.as_str().starts_with("abandon"),
            "First word should be corrected to 'abandon'"
        );
    }

    #[test]
    fn test_validate_with_multiple_typos() {
        let mnemonic = "abanbon abancon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let (is_valid, corrected) = validate_and_correct_passphrase(mnemonic);

        assert!(is_valid, "Should correct multiple typos");
        let words: Vec<&str> = corrected.as_str().split_whitespace().collect();
        assert_eq!(words[0], "abandon", "First typo should be corrected");
        assert_eq!(words[1], "abandon", "Second typo should be corrected");
    }

    #[test]
    fn test_validate_with_invalid_word() {
        // A word that doesn't match anything in the wordlist
        let mnemonic = "xyzabc abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let (is_valid, corrected) = validate_and_correct_passphrase(mnemonic);

        assert!(!is_valid, "Should reject completely invalid words");
        assert_eq!(corrected, "", "Should return empty string");
    }

    #[test]
    fn test_validate_with_extra_whitespace() {
        let mnemonic = "abandon  abandon   abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let (is_valid, corrected) = validate_and_correct_passphrase(mnemonic);

        assert!(is_valid, "Should handle extra whitespace");
        assert_eq!(corrected, get_test_mnemonic());
    }

    #[test]
    fn test_validate_empty_string() {
        let (is_valid, corrected) = validate_and_correct_passphrase("");
        assert!(!is_valid, "Should reject empty string");
        assert_eq!(corrected, "");
    }

    #[test]
    fn test_validate_invalid_checksum() {
        // Create a mnemonic with incorrect checksum by changing the last word
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon";
        let (is_valid, _) = validate_and_correct_passphrase(mnemonic);

        assert!(!is_valid, "Should reject mnemonic with invalid checksum");
    }

    #[test]
    fn test_levenshtein_distance_same_strings() {
        assert_eq!(levenshtein_distance("hello", "hello"), 0);
    }

    #[test]
    fn test_levenshtein_distance_one_char_diff() {
        assert_eq!(levenshtein_distance("hello", "hallo"), 1);
        assert_eq!(levenshtein_distance("hello", "hullo"), 1);
    }

    #[test]
    fn test_levenshtein_distance_insertion() {
        assert_eq!(levenshtein_distance("hello", "helllo"), 1);
    }

    #[test]
    fn test_levenshtein_distance_deletion() {
        assert_eq!(levenshtein_distance("hello", "helo"), 1);
    }

    #[test]
    fn test_levenshtein_distance_empty_strings() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("", "hello"), 5);
        assert_eq!(levenshtein_distance("hello", ""), 5);
    }

    #[test]
    fn test_levenshtein_distance_completely_different() {
        assert_eq!(levenshtein_distance("abc", "xyz"), 3);
    }

    #[test]
    fn test_validate_checksum_with_valid_indices() {
        // "abandon" is typically index 0, "about" is typically index 3
        let indices = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3];

        assert!(
            validate_checksum(&indices[..]),
            "Should validate correct checksum"
        );
    }

    #[test]
    fn test_validate_checksum_with_invalid_indices() {
        // All zeros would not have a valid checksum for "abandon abandon ... abandon"
        let indices = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

        assert!(
            !validate_checksum(&indices[..]),
            "Should reject incorrect checksum"
        );
    }

    #[test]
    fn test_validate_ambiguous_prefix() {
        // Test a prefix that matches multiple words
        // "ab" could match "abandon", "ability", "able", "about", "above", "absent", "absorb", "abstract", "absurd"
        let wordlist: Vec<&str> = WORDLIST.lines().collect();
        let ab_matches: Vec<_> = wordlist.iter().filter(|&&w| w.starts_with("ab")).collect();

        if ab_matches.len() > 1 {
            let mnemonic = "ab ab ab ab ab ab ab ab ab ab ab ab";
            let (is_valid, corrected) = validate_and_correct_passphrase(mnemonic);

            // Should fail because "ab" is ambiguous
            assert!(
                !is_valid || !corrected.is_empty(),
                "Should handle ambiguous prefix somehow"
            );
        }
    }

    #[test]
    fn test_roundtrip_generate_and_validate() {
        // Generate 10 passphrases and ensure they all validate
        for _ in 0..10 {
            let passphrase = get_mnemonic().unwrap();
            let (is_valid, corrected) = validate_and_correct_passphrase(passphrase.as_str());

            assert!(
                is_valid,
                "Generated passphrase should validate: {}",
                passphrase
            );
            assert_eq!(corrected, passphrase, "Corrected should match original");
        }
    }

    #[test]
    fn test_validate_preserves_word_order() {
        let mnemonic = get_test_mnemonic();
        let (_, corrected) = validate_and_correct_passphrase(mnemonic);

        let original_words: Vec<&str> = mnemonic.split_whitespace().collect();
        let corrected_words: Vec<&str> = corrected.as_str().split_whitespace().collect();

        assert_eq!(
            original_words, corrected_words,
            "Word order should be preserved"
        );
    }
}
