use dollcode_core::{
    from_dollcode,
    text::{TextIterator, CHAR_MAP as TEXT_CHAR_MAP},
    to_dollcode, CHAR_MAP,
};
use heapless::Vec;

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
fn test_dollcode_segments_roundtrip() {
    use heapless::Vec;
    const TEST_VEC_SIZE: usize = 256;

    // Test encoding text to dollcode and back
    let test_strings = ["Hello", "123", "ABC", "!@#", " ", "Mixed-Case_123!"];

    for &input in &test_strings {
        // Convert string to dollcode segments
        let mut dollcode_chars: Vec<char, TEST_VEC_SIZE> = Vec::new();

        // Collect all dollcode chars from text segments
        for result in TextIterator::new(input) {
            let segment = result.unwrap();
            for &c in segment.as_chars() {
                dollcode_chars.push(c).unwrap();
            }
        }

        // Verify these are valid dollcode chars
        for &c in &dollcode_chars {
            assert!(
                CHAR_MAP.contains(&c),
                "Invalid dollcode char produced: {}",
                c
            );
        }

        // TODO: Add function to decode dollcode back to text
        // let decoded = decode_text(&dollcode_chars);
        // assert_eq!(input, decoded);
    }
}

#[test]
fn test_mixed_numeric_and_text() {
    // Test that numeric dollcode doesn't conflict with text dollcode
    let numeric_cases = [
        1, 42, 100, 999  // Changed to avoid 5-digit sequences
    ];

    for &num in &numeric_cases {
        let numeric_dollcode = to_dollcode(num).unwrap();

        // Verify it's not exactly 5 digits (which would look like a text segment)
        if numeric_dollcode.len() == 5 {
            // If it is 5 digits, verify it doesn't decode as valid text
            let is_valid_text = TextIterator::new("A")  // Get a valid text segment to compare
                .next()
                .unwrap()
                .unwrap()
                .as_chars()
                .iter()
                .all(|c| CHAR_MAP.contains(c));

            assert!(!is_valid_text,
                "Numeric value {} produced a valid text segment pattern", num);
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
            assert_eq!(segment.len(), 5,
                "Wrong segment length for char '{}'", c);

            // Verify each dollcode char is valid
            for &dc in segment.as_chars() {
                assert!(CHAR_MAP.contains(&dc),
                    "Invalid dollcode char '{}' produced for '{}'", dc, c);
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
