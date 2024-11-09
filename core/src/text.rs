//! Text encoding and decoding functionality for dollcode.
//!
//! This module provides zero-allocation text handling by mapping ASCII characters
//! to 5-digit dollcode sequences. Characters are mapped to positions 0-94 according
//! to a standardized order, then each position is encoded in base-3 using dollcode
//! characters.
//!
//! # Character Mapping
//!
//! Characters are mapped to positions in the following order:
//! - Uppercase letters (A-Z): positions 0-25
//! - Lowercase letters (a-z): positions 26-51
//! - Numbers (0-9): positions 52-61
//! - Space: position 62
//! - Punctuation and symbols: positions 63-94
//!
//! # Examples
//!
//! ```rust
//! use dollcode_core::text::TextIterator;
//!
//! let text = "Hi!";
//! for result in TextIterator::new(text) {
//!     let segment = result.unwrap();
//!     // Each character becomes a 5-digit sequence
//!     assert_eq!(segment.len(), 5);
//!     assert_eq!(segment.as_chars().len(), 5);
//!     // Verify only valid dollcode characters are produced
//!     for &c in segment.as_chars() {
//!         assert!(matches!(c, '▖' | '▘' | '▌'));
//!     }
//! }
//! ```

use crate::{error::DollcodeError, error::Result, CHAR_MAP as DOLLCODE_CHARS};
use core::str::Chars;

/// Map of printable ASCII characters in encoding order
pub const CHAR_MAP: &[char] = &[
    // 0-25: Uppercase letters
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z', // 26-51: Lowercase letters
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z', // 52-61: Numbers
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', // 62: Space
    ' ', // 63-94: Punctuation and symbols
    '!', '"', '#', '$', '%', '&', '\'', '(', ')', '*', '+', ',', '-', '.', '/', ':', ';', '<', '=',
    '>', '?', '@', '[', '\\', ']', '^', '_', '`', '{', '|', '}', '~',
];

/// Each character is encoded as 5 dollcode digits (need 5 base-3 digits for 95 chars)
pub const SEGMENT_SIZE: usize = 5;

/// A fixed-length dollcode sequence representing a single character
#[derive(Debug, Clone, Copy)]
pub struct TextSegment {
    /// Buffer holding the dollcode characters
    chars: [char; SEGMENT_SIZE],
    /// Number of valid characters (always SEGMENT_SIZE for valid text)
    len: usize,
}

impl Default for TextSegment {
    fn default() -> Self {
        Self {
            chars: ['\0'; SEGMENT_SIZE],
            len: 0,
        }
    }
}

impl TextSegment {
    /// Creates an empty text segment
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a slice of the valid characters in this segment
    #[inline]
    pub fn as_chars(&self) -> &[char] {
        &self.chars[..self.len]
    }

    /// Returns the number of characters in this segment
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns true if this segment is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// Zero-allocation iterator over text, producing dollcode segments
pub struct TextIterator<'a> {
    /// Source character iterator
    chars: Chars<'a>,
    /// Current position in input (for error reporting)
    position: usize,
    /// Working buffer for current segment
    buffer: [char; SEGMENT_SIZE],
    /// Number of chars in buffer
    buffer_len: usize,
}

impl<'a> TextIterator<'a> {
    #[inline]
    pub fn new(text: &'a str) -> Self {
        Self {
            chars: text.chars(),
            position: 0,
            buffer: ['\0'; SEGMENT_SIZE],
            buffer_len: 0,
        }
    }

    #[inline]
    fn process_char(&mut self, c: char) -> Result<Option<TextSegment>> {
        let pos = self.position;
        self.position += 1;

        // Find char index in mapping
        let value = CHAR_MAP
            .iter()
            .position(|&x| x == c)
            .ok_or(DollcodeError::InvalidChar(c, pos))?;

        // Convert to base-3 digits
        let mut n = value;
        for i in (0..SEGMENT_SIZE).rev() {
            self.buffer[i] = DOLLCODE_CHARS[n % 3];
            n /= 3;
        }
        self.buffer_len = SEGMENT_SIZE;

        let mut segment = TextSegment::new();
        segment.chars.copy_from_slice(&self.buffer);
        segment.len = self.buffer_len;

        Ok(Some(segment))
    }
}

impl<'a> Iterator for TextIterator<'a> {
    type Item = Result<TextSegment>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.chars.next() {
            Some(c) => match self.process_char(c) {
                Ok(Some(segment)) => Some(Ok(segment)),
                Ok(None) => None,
                Err(e) => Some(Err(e)),
            },
            None => None,
        }
    }
}

/// Decodes dollcode segments back into text characters
pub fn decode_text_segment(chars: &[char]) -> Result<char> {
    // A text segment must be exactly SEGMENT_SIZE chars
    if chars.len() != SEGMENT_SIZE {
        return Err(DollcodeError::InvalidInput);
    }

    // Convert dollcode chars back to base-3 value
    let mut value = 0usize;
    for &c in chars {
        // First multiply by base
        value = value.checked_mul(3)
            .ok_or(DollcodeError::Overflow)?;

        // Then add digit value
        let digit = match c {
            '▖' => 0,  // Changed to 0-based values
            '▘' => 1,
            '▌' => 2,
            _ => return Err(DollcodeError::InvalidInput),
        };

        value = value.checked_add(digit)
            .ok_or(DollcodeError::Overflow)?;
    }

    // Get the character from our mapping
    // Note: value is now 0-based so we don't subtract 1
    CHAR_MAP.get(value)
        .copied()
        .ok_or(DollcodeError::InvalidInput)
}

/// Iterator that decodes dollcode segments back into text
pub struct TextDecoder<'a> {
    /// Source dollcode characters
    chars: core::slice::Chunks<'a, char>,
    /// Position for error reporting
    position: usize,
}

impl<'a> TextDecoder<'a> {
    /// Creates a new text decoder from dollcode characters
    #[inline]
    pub fn new(chars: &'a [char]) -> Self {
        Self {
            chars: chars.chunks(SEGMENT_SIZE),
            position: 0,
        }
    }
}

impl<'a> Iterator for TextDecoder<'a> {
    type Item = Result<char>;

    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.chars.next()?;
        self.position += 1;
        Some(decode_text_segment(chunk))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use heapless::Vec;

    const TEST_VEC_SIZE: usize = 32;

    #[test]
    fn test_ascii_categories() {
        // Test each ASCII category has correct encoding
        let test_cases = [
            // Uppercase
            'A', 'M', 'Z', // Lowercase
            'a', 'm', 'z', // Numbers
            '0', '5', '9', // Space
            ' ', // Punctuation
            '.', '!', '@', '#', // Symbols
            '+', '=', '/', '\\',
        ];

        let mut segments: Vec<TextSegment, TEST_VEC_SIZE> = Vec::new();

        for &c in &test_cases {
            // Pass the char directly as a str slice
            let bytes = [c as u8];
            let s = core::str::from_utf8(&bytes).unwrap();
            let mut iter = TextIterator::new(s);
            if let Some(Ok(segment)) = iter.next() {
                segments.extend_from_slice(&[segment]).unwrap();
                assert_eq!(segment.len, SEGMENT_SIZE);
            }
        }
    }

    #[test]
    fn test_invalid_chars() {
        // Raw bytes for the invalid chars
        let invalid = [
            &[0x0A][..],             // \n
            &[0x09][..],             // \t
            &[0xB1][..],             // ±
            &[0xE2, 0x82, 0xAC][..], // €
            &[0xE2, 0xAD, 0x90][..], // ⭐
        ];

        for bytes in invalid {
            if let Ok(s) = core::str::from_utf8(bytes) {
                let mut iter = TextIterator::new(s);
                match iter.next() {
                    Some(Err(DollcodeError::InvalidChar(_, _))) => (),
                    _ => panic!("Expected invalid char error"),
                }
            }
        }
    }

    #[test]
    fn test_sequence_length() {
        // Every text character should encode to exactly SEGMENT_SIZE dollcode chars
        let text = "Hello, World!";
        let mut segments: Vec<TextSegment, TEST_VEC_SIZE> = Vec::new();

        for result in TextIterator::new(text) {
            let segment = result.unwrap();
            segments.extend_from_slice(&[segment]).unwrap();
            assert_eq!(segment.len, SEGMENT_SIZE);

            // Verify each char is a valid dollcode character
            for &c in segment.as_chars() {
                assert!(
                    DOLLCODE_CHARS.contains(&c),
                    "Expected dollcode character, got: {}",
                    c
                );
            }
        }

        assert_eq!(segments.len(), text.len());
    }

    #[test]
    fn test_char_positions() {
        // Verify character mapping positions match expected order
        assert_eq!(CHAR_MAP.iter().position(|&c| c == 'A'), Some(0));
        assert_eq!(CHAR_MAP.iter().position(|&c| c == 'Z'), Some(25));
        assert_eq!(CHAR_MAP.iter().position(|&c| c == 'a'), Some(26));
        assert_eq!(CHAR_MAP.iter().position(|&c| c == 'z'), Some(51));
        assert_eq!(CHAR_MAP.iter().position(|&c| c == '0'), Some(52));
        assert_eq!(CHAR_MAP.iter().position(|&c| c == '9'), Some(61));
        assert_eq!(CHAR_MAP.iter().position(|&c| c == ' '), Some(62));
    }

    #[test]
    fn test_consistent_encoding() {
        // Same character should always encode to same dollcode sequence
        let text = "AA"; // Same character repeated
        let mut iter = TextIterator::new(text);

        if let (Some(Ok(first)), Some(Ok(second))) = (iter.next(), iter.next()) {
            assert_eq!(first.as_chars(), second.as_chars());
        } else {
            panic!("Failed to get segments");
        }
    }

    #[test]
    fn test_capacity() {
        let long_text = "This is a longer text that should still work within fixed buffers!";
        let mut count = 0;

        for result in TextIterator::new(long_text) {
            let segment = result.unwrap();
            assert_eq!(segment.len, SEGMENT_SIZE);
            count += 1;
        }

        assert_eq!(count, long_text.len());
    }

    #[test]
    fn test_decode_text_segment() {
        // Test decoding individual segments
        for &c in CHAR_MAP {
            // Create single-char input
            let bytes = [c as u8];
            let input = core::str::from_utf8(&bytes).unwrap();

            // First encode
            let mut iter = TextIterator::new(input);
            let segment = iter.next().unwrap().unwrap();

            // Then decode
            let decoded = decode_text_segment(segment.as_chars()).unwrap();

            assert_eq!(c, decoded,
                "Character '{}' did not round-trip correctly", c);
        }
    }

    #[test]
    fn test_text_decoder() {
        let test_strings = [
            "A",  // Start with simple single char
            "B",
            "1",
            "!",
            "AB",  // Then try pairs
            "12",
            "Hello",  // Then longer strings
            "123!@#",
        ];

        for &input in &test_strings {
            // First encode the string
            let mut encoded: Vec<char, 128> = Vec::new();
            for result in TextIterator::new(input) {
                let segment = result.unwrap();
                encoded.extend_from_slice(segment.as_chars()).unwrap();
            }

            // Then decode back
            let mut decoded = heapless::String::<128>::new();
            for result in TextDecoder::new(&encoded) {
                decoded.push(result.unwrap()).unwrap();
            }

            assert_eq!(input, decoded.as_str(),
                "String '{}' did not round-trip correctly", input);
        }
    }

    #[test]
    fn test_single_chars() {
        // Test each character individually
        for &c in CHAR_MAP {
            // Encode
            let bytes = [c as u8];
            let input = core::str::from_utf8(&bytes).unwrap();
            let mut encoded: Vec<char, 32> = Vec::new();

            for result in TextIterator::new(input) {
                let segment = result.unwrap();
                encoded.extend_from_slice(segment.as_chars()).unwrap();
            }

            // Decode
            let decoded = decode_text_segment(&encoded).unwrap();
            assert_eq!(c, decoded);
        }
    }

    #[test]
    fn test_invalid_decode() {
        // Test invalid segment sizes
        let short_segment = ['▖', '▘', '▌', '▖'];
        assert!(matches!(
            decode_text_segment(&short_segment),
            Err(DollcodeError::InvalidInput)
        ));

        // Test invalid characters
        let invalid_chars = ['A', 'B', 'C', 'D', 'E'];
        assert!(matches!(
            decode_text_segment(&invalid_chars),
            Err(DollcodeError::InvalidInput)
        ));

        // Test segment with mixed valid/invalid chars
        let mixed_chars = ['▖', '▘', 'A', '▌', '▖'];
        assert!(matches!(
            decode_text_segment(&mixed_chars),
            Err(DollcodeError::InvalidInput)
        ));
    }
}
