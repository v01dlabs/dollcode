use dollcode::{
    from_dollcode,
    text::{TextDecoder, TextIterator, DELIMITER},
    to_dollcode, DOLLCODE_CHAR_MAP,
};
use heapless::{String, Vec};

const TEST_VEC_SIZE: usize = 256;

// Helper function to check if a character is valid dollcode
fn is_valid_dollcode(c: char) -> bool {
    DOLLCODE_CHAR_MAP.contains(&c) || c == DELIMITER
}

#[test]
fn test_text_to_dollcode_validation() {
    let text = "Hello!";

    for result in TextIterator::new(text) {
        let segment = result.unwrap();
        for &c in segment.as_chars() {
            assert!(
                is_valid_dollcode(c),
                "TextIterator produced invalid dollcode char: {}",
                c
            );
        }
    }
}

#[test]
fn test_valid_dollcode_for_all_chars() {
    for ascii_val in 32..=126 {
        let c = char::from_u32(ascii_val).unwrap();
        let c_string = c.to_string();
        let mut iter = TextIterator::new(&c_string);
        let segment = iter.next().unwrap().unwrap();

        for &dc in segment.as_chars() {
            assert!(
                is_valid_dollcode(dc),
                "Invalid dollcode char {} for input {}",
                dc,
                c
            );
        }
    }
}

#[test]
fn test_text_encoding_decoding_roundtrip() {
    let test_strings = [
        "Hello, World!",
        "123-456-789",
        "UPPER lower 12345",
        "!@#$%^&*()",
        "Mixed_Case_With_Numbers_123",
        "AAAAA", // repeated chars
        "     ", // all spaces
        "!!!!!", // repeated symbols
        "12321", // palindrome
        "aA1!@", // one of each type
    ];

    for &input in &test_strings {
        // First encode to dollcode
        let mut encoded: Vec<char, 512> = Vec::new();
        for result in TextIterator::new(input) {
            let segment = result.unwrap();
            encoded.extend_from_slice(segment.as_chars()).unwrap();
        }

        // Verify all encoded chars are valid
        for &c in &encoded {
            assert!(
                is_valid_dollcode(c),
                "Invalid dollcode char produced: {}",
                c
            );
        }

        // Then decode back
        let mut decoded = String::<512>::new();
        let encoded_str: String<512> = encoded.iter().collect();
        for result in TextDecoder::new(&encoded_str) {
            decoded.push(result.unwrap()).unwrap();
        }

        assert_eq!(
            input,
            decoded.as_str(),
            "Failed roundtrip for string: {}",
            input
        );
    }
}

#[test]
fn test_mixed_numeric_and_text() {
    let numeric_cases = [1, 42, 100, 999];

    for &num in &numeric_cases {
        let numeric_dollcode = to_dollcode(num).unwrap();

        let roundtrip = from_dollcode(numeric_dollcode.as_chars()).unwrap();
        assert_eq!(num, roundtrip);
    }
}

#[test]
fn test_all_ascii_printable() {
    for ascii_val in 32..=126 {
        let c = char::from_u32(ascii_val).unwrap();
        let test_str = &c.to_string();
        let mut iter = TextIterator::new(test_str);

        if let Some(Ok(segment)) = iter.next() {
            assert!(
                segment.len() >= 3,
                "Segment length must be at least 3 for char '{}'",
                c
            );

            for &dc in segment.as_chars() {
                assert!(
                    is_valid_dollcode(dc),
                    "Invalid dollcode char '{}' produced for '{}'",
                    dc,
                    c
                );
            }
        } else {
            panic!("Failed to encode character: {}", c);
        }
    }
}

#[test]
fn test_special_sequences() {
    let test_cases = [
        "AaAaAa",              // Alternating case
        "111222333",           // Repeated numbers
        "!@#$%^&*()",          // Special characters
        "     ",               // Multiple spaces
        "aA1!aA1!",            // Repeating pattern
        "The quick brown fox", // Partial pangram
        "HELLO, WORLD!",       // All caps with punctuation
        "12345-67890",         // Numbers with separator
        "@#$%^&",              // Consecutive symbols
        "MiXeD_CaSe_123",      // Mixed case with numbers and underscore
    ];

    for &test_str in &test_cases {
        let mut dollcode_chars: Vec<char, TEST_VEC_SIZE> = Vec::new();

        for result in TextIterator::new(test_str) {
            let segment = result.unwrap();
            assert!(segment.len() >= 3, "Segment too short for '{}'", test_str);

            for &c in segment.as_chars() {
                dollcode_chars.push(c).unwrap();
                assert!(
                    is_valid_dollcode(c),
                    "Invalid dollcode produced for '{}': {}",
                    test_str,
                    c
                );
            }
        }
    }
}

#[test]
fn test_edge_patterns() {
    let patterns = [
        "A",     // Single uppercase
        "z",     // Single lowercase
        "9",     // Single number
        " ",     // Single space
        "~",     // Last ASCII char we support
        "!",     // First special char
        "Aa1! ", // One of each category
        "ZZZZZ", // Repeated last uppercase
        "aaaaa", // Repeated lowercase
        "99999", // Repeated numbers
        "     ", // Repeated spaces
        "!!!!!", // Repeated specials
    ];

    for &pattern in &patterns {
        let mut encoded: Vec<char, TEST_VEC_SIZE> = Vec::new();

        for result in TextIterator::new(pattern) {
            let segment = result.unwrap();
            assert!(segment.len() >= 3, "Segment too short for '{}'", pattern);

            for &c in segment.as_chars() {
                encoded.push(c).unwrap();
                assert!(
                    is_valid_dollcode(c),
                    "Invalid dollcode for '{}': {}",
                    pattern,
                    c
                );
            }
        }
    }
}

#[test]
fn test_text_decode_errors() {
    let invalid_cases = [
        // Wrong segment size (less than minimum 3)
        &['▖', '▘'][..],
        // Invalid characters mixed with valid ones
        &['A', '▖', '▘', '▌'][..],
        &['▖', 'B', '▘', '▌'][..],
        &['▖', '▘', '▌', 'C'][..],
        // Empty input
        &[][..],
        // Non-dollcode characters
        &['1', '2', '3', '4'][..],
    ];

    for &invalid in &invalid_cases {
        let invalid_string: String<32> = String::from_iter(invalid.iter().copied());
        let mut decoder = TextDecoder::new(&invalid_string);
        match decoder.next() {
            None => {
                assert_eq!(invalid.len(), 0, "Only empty input should give None");
            }
            Some(Ok(c)) => {
                panic!(
                    "Expected error but got char: {} from input: {:?}",
                    c, invalid
                );
            }
            Some(Err(_)) => {
                // This is what is expected for invalid inputs
            }
        }
    }
}

#[test]
fn test_mixed_encoding_patterns() {
    let test_cases = [
        ("A", 1),
        ("B", 42),
        ("123", 999),
        ("ABC", 12345),
        ("!!!", 98765),
    ];

    for &(text, number) in &test_cases {
        // Encode text
        let mut text_encoded: Vec<char, 128> = Vec::new();
        for result in TextIterator::new(text) {
            let segment = result.unwrap();
            text_encoded.extend_from_slice(segment.as_chars()).unwrap();
        }

        // Encode number
        let num_encoded = to_dollcode(number).unwrap();

        // Verify text decodes correctly
        let mut decoded = String::<128>::new();
        let text_encoded_str: String<128> = text_encoded.iter().collect();
        for result in TextDecoder::new(&text_encoded_str) {
            decoded.push(result.unwrap()).unwrap();
        }
        assert_eq!(text, decoded.as_str());

        // Verify number decodes correctly
        let num_decoded = from_dollcode(num_encoded.as_chars()).unwrap();
        assert_eq!(number, num_decoded);
    }
}
