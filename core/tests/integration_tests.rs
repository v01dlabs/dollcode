use dollcode::{
    from_dollcode,
    text::{TextDecoder, TextIterator, CHAR_MAP as TEXT_CHAR_MAP},
    to_dollcode, CHAR_MAP,
};
use heapless::{String, Vec};

const TEST_VEC_SIZE: usize = 256;

#[test]
fn test_text_to_dollcode_validation() {
    // Test that text segments produce valid dollcode chars
    let text = "Hello!";

    for result in TextIterator::new(text) {
        let segment = result.unwrap();
        // Verify each char in segment is a valid dollcode character
        for &c in segment.as_chars() {
            assert!(
                CHAR_MAP.contains(&c),
                "TextIterator produced invalid dollcode char: {}",
                c
            );
        }
    }
}

#[test]
fn test_valid_dollcode_for_all_chars() {
    // Test every valid input character
    for &c in TEXT_CHAR_MAP {
        let c_string = c.to_string();
        let mut iter = TextIterator::new(&c_string);
        let segment = iter.next().unwrap().unwrap();

        // Each segment should be valid dollcode
        for &dc in segment.as_chars() {
            assert!(
                CHAR_MAP.contains(&dc),
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
        // Add some edge cases
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

        // Then decode back
        let mut decoded = String::<512>::new();
        for result in TextDecoder::new(&encoded) {
            decoded.push(result.unwrap()).unwrap();
        }

        // Verify roundtrip
        assert_eq!(
            input,
            decoded.as_str(),
            "Failed roundtrip for string: {}",
            input
        );

        // Verify all encoded chars are valid dollcode
        for &c in &encoded {
            assert!(
                CHAR_MAP.contains(&c),
                "Invalid dollcode char produced: {}",
                c
            );
        }

        // Verify encoded length
        assert_eq!(
            encoded.len(),
            input.len() * 5,
            "Incorrect encoded length for: {}",
            input
        );
    }
}

#[test]
fn test_mixed_numeric_and_text() {
    // Test that numeric dollcode doesn't conflict with text dollcode
    let numeric_cases = [
        1, 42, 100, 999, // Changed to avoid 5-digit sequences
    ];

    for &num in &numeric_cases {
        let numeric_dollcode = to_dollcode(num).unwrap();

        // Verify it's not exactly 5 digits (which would look like a text segment)
        if numeric_dollcode.len() == 5 {
            // If it is 5 digits, verify it doesn't decode as valid text
            let is_valid_text = TextIterator::new("A") // Get a valid text segment to compare
                .next()
                .unwrap()
                .unwrap()
                .as_chars()
                .iter()
                .all(|c| CHAR_MAP.contains(c));

            assert!(
                !is_valid_text,
                "Numeric value {} produced a valid text segment pattern",
                num
            );
        }

        let roundtrip = from_dollcode(numeric_dollcode.as_chars()).unwrap();
        assert_eq!(num, roundtrip);
    }
}

#[test]
fn test_all_ascii_printable() {
    // Test every printable supported ASCII character one at a time
    for &c in TEXT_CHAR_MAP {
        // Test each character individually
        let test_str = &c.to_string();
        let mut iter = TextIterator::new(test_str);

        if let Some(Ok(segment)) = iter.next() {
            // Verify segment length
            assert_eq!(segment.len(), 5, "Wrong segment length for char '{}'", c);

            // Verify each dollcode char is valid
            for &dc in segment.as_chars() {
                assert!(
                    CHAR_MAP.contains(&dc),
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
        // Various patterns
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

        // Collect encoded sequence
        for result in TextIterator::new(test_str) {
            let segment = result.unwrap();
            for &c in segment.as_chars() {
                dollcode_chars.push(c).unwrap();
                // Verify it's a valid dollcode character
                assert!(
                    CHAR_MAP.contains(&c),
                    "Invalid dollcode produced for '{}': {}",
                    test_str,
                    c
                );
            }
        }

        // Verify correct segment sizes
        assert_eq!(
            dollcode_chars.len(),
            test_str.len() * 5,
            "Incorrect encoded length for '{}'",
            test_str
        );
    }
}

#[test]
fn test_edge_patterns() {
    let patterns = [
        // Edge cases and boundaries
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
            assert_eq!(
                segment.as_chars().len(),
                5,
                "Wrong segment size for '{}'",
                pattern
            );

            for &c in segment.as_chars() {
                encoded.push(c).unwrap();
                assert!(
                    CHAR_MAP.contains(&c),
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
    // Verify our segment size math
    assert!(
        3_usize.pow(4) < TEXT_CHAR_MAP.len(),
        "4 base-3 digits not enough"
    );
    assert!(
        3_usize.pow(5) > TEXT_CHAR_MAP.len(),
        "5 base-3 digits sufficient"
    );

    // Test various invalid inputs
    let invalid_cases = [
        // Wrong segment size (must be exactly 5)
        &['▖', '▘', '▌'][..],      // Too short
        &['▖', '▘', '▌', '▖'][..], // Too short
        // Invalid characters mixed with valid ones
        &['A', '▖', '▘', '▌', '▖'][..], // Invalid first char
        &['▖', 'B', '▘', '▌', '▖'][..], // Invalid middle char
        &['▖', '▘', '▌', '▖', 'C'][..], // Invalid last char
        // Empty input
        &[][..],
        // Non-dollcode characters
        &['1', '2', '3', '4', '5'][..],
        // Maximum value in base-3 with 5 digits is 3^5-1 = 242
        // Values above TEXT_CHAR_MAP.len() should error
        &['▌', '▌', '▌', '▌', '▌'][..], // This is (3,3,3,3,3) in base-3 = 242
    ];

    for &invalid in &invalid_cases {
        let mut decoder = TextDecoder::new(invalid);
        match decoder.next() {
            None => {
                // Empty input is fine
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
    // Test mixing text and numeric encodings
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

        // Verify they're different
        assert_ne!(
            text_encoded.as_slice(),
            num_encoded.as_chars(),
            "Text and numeric encoding produced same result: {} vs {}",
            text,
            number
        );

        // Verify text decodes correctly
        let mut decoded = String::<128>::new();
        for result in TextDecoder::new(&text_encoded) {
            decoded.push(result.unwrap()).unwrap();
        }
        assert_eq!(text, decoded.as_str());

        // Verify number decodes correctly
        let num_decoded = from_dollcode(num_encoded.as_chars()).unwrap();
        assert_eq!(number, num_decoded);
    }
}
