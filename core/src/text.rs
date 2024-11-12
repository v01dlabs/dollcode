use crate::{DollcodeError, Result};
use core::result::Result as CoreResult;
use core::{iter::Peekable, str::Chars};

/// A fixed-size text segment representing encoded dollcode characters.
///
/// Each segment contains the dollcode representation of a single ASCII character,
/// using a fixed-size internal buffer to maintain zero-allocation guarantees.
///
/// # Examples
///
/// ```rust
/// # use dollcode::text::TextSegment;
/// let segment = TextSegment::new();
/// assert!(segment.is_empty());
/// assert_eq!(segment.len(), 0);
/// ```
#[derive(Debug, Copy, Clone)]
pub struct TextSegment {
    chars: [char; 6],
    len: usize,
}

impl TextSegment {
    /// Returns the number of valid characters in this segment.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns true if this segment contains no characters.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Default for TextSegment {
    fn default() -> Self {
        Self::new()
    }
}

impl TextSegment {
    /// Creates a new empty text segment.
    ///
    /// The segment is initialized with a zeroed buffer and can hold
    /// up to 6 characters (5 dollcode characters + delimiter).
    #[inline]
    pub fn new() -> Self {
        Self {
            chars: ['\0'; 6],
            len: 0,
        }
    }

    /// Returns a slice of the valid characters in this segment.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use dollcode::{error::Result, text::TextIterator};
    /// # fn main() -> Result<()> {
    /// let mut iter = TextIterator::new("A");
    /// let segment = iter.next().unwrap()?;
    /// assert!(!segment.as_chars().is_empty());
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn as_chars(&self) -> &[char] {
        &self.chars[..self.len]
    }

    /// Pushes a character onto this segment.
    ///
    /// # Errors
    ///
    /// Returns [`DollcodeError::Overflow`] if the segment is full.
    #[inline]
    fn push(&mut self, c: char) -> Result<()> {
        if self.len >= self.chars.len() {
            return Err(DollcodeError::Overflow);
        }
        self.chars[self.len] = c;
        self.len += 1;
        Ok(())
    }
}

/// Zero-allocation iterator that converts ASCII text into dollcode segments.
///
/// This iterator processes input text character by character, converting each ASCII
/// character into a unique sequence of dollcode characters. The conversion maintains
/// zero-allocation guarantees by using fixed-size buffers.
///
/// # Examples
///
/// ```rust
/// # use dollcode::{error::Result, text::TextIterator};
/// # fn main() -> Result<()> {
/// let text = "Hello!";
/// let mut encoded = heapless::Vec::<char, 128>::new();
///
/// for result in TextIterator::new(text) {
///     let segment = result?;
///     encoded.extend_from_slice(segment.as_chars()).unwrap();
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct TextIterator<'a> {
    chars: Peekable<Chars<'a>>,
    position: usize,
}

impl<'a> TextIterator<'a> {
    /// Creates a new text iterator from the input string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use dollcode::text::TextIterator;
    /// let iter = TextIterator::new("Hello");
    /// ```
    pub fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
            position: 0,
        }
    }

    /// Processes a single character into a dollcode segment.
    ///
    /// This function converts an ASCII character into its dollcode representation by:
    /// 1. Validating the character is in the ASCII printable range (32-126)
    /// 2. Converting to base-3 digits
    /// 3. Mapping digits to dollcode characters
    /// 4. Padding to a consistent length
    ///
    /// # Errors
    ///
    /// Returns [`DollcodeError::InvalidChar`] if the character is outside the valid ASCII range.
    #[inline]
    fn process_char(&mut self, c: char) -> Result<TextSegment> {
        let pos = self.position;
        self.position += 1;

        // Only accept ASCII
        let code = c as u32;
        if !(32..=126).contains(&code) {
            return Err(DollcodeError::InvalidChar(c, pos));
        }

        let mut segment = TextSegment::new();
        let mut num = code;
        let mut digits = [0u8; 8];
        let mut idx = 0;

        // Convert to bijective base-3
        while num > 0 && idx < 8 {
            let rem = num % 3;
            let digit = if rem == 0 { 3 } else { rem as u8 };
            num = if rem == 0 { num / 3 - 1 } else { num / 3 };
            digits[idx] = digit;
            idx += 1;
        }

        // Reverse digits and map to dollcode characters
        for &digit in digits[..idx].iter().rev() {
            segment.push(match digit {
                1 => '▖',
                2 => '▘',
                3 => '▌',
                _ => return Err(DollcodeError::InvalidInput),
            })?;
        }

        // Pad to minimum length for consistent decoding
        while segment.len() < 3 {
            segment.push('▖')?;
        }

        Ok(segment)
    }
}

/// Zero-width joiner character used as a delimiter between dollcode segments.
pub const DELIMITER: char = '\u{200D}';

impl<'a> Iterator for TextIterator<'a> {
    type Item = Result<TextSegment>;

    fn next(&mut self) -> Option<Self::Item> {
        self.chars.next().map(|c| {
            let mut segment = self.process_char(c)?;
            segment.push(DELIMITER)?;

            Ok(segment)
        })
    }
}

/// Zero-allocation iterator that converts dollcode back into ASCII text.
///
/// This iterator processes dollcode sequences in groups, converting each valid
/// group back into its corresponding ASCII character. The decoding process
/// maintains zero-allocation guarantees and validates input sequences.
///
/// # Examples
///
/// ```rust
/// # use dollcode::{error::Result, text::TextDecoder};
/// # fn main() -> Result<()> {
/// let dollcode = "▖▖▖▌";  // Valid dollcode sequence
/// let mut decoded = String::new();
///
/// for result in TextDecoder::new(dollcode) {
///     decoded.push(result?);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct TextDecoder<'a> {
    segments: Peekable<core::str::Split<'a, char>>,
    position: usize,
}

impl<'a> TextDecoder<'a> {
    /// Creates a new decoder from dollcode input.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use dollcode::text::TextDecoder;
    /// let decoder = TextDecoder::new("▖▘▌");
    /// ```
    pub fn new(encoded: &'a str) -> Self {
        Self {
            segments: encoded.split(DELIMITER).peekable(),
            position: 0,
        }
    }
}

impl<'a> Iterator for TextDecoder<'a> {
    type Item = CoreResult<char, DollcodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        let segment = match self.segments.next() {
            Some(seg) if !seg.is_empty() => seg,
            _ => return None, // Skip empty segments
        };

        let mut value: u32 = 0;

        for c in segment.chars() {
            let digit = match c {
                '▖' => 1,
                '▘' => 2,
                '▌' => 3,
                _ => return Some(Err(DollcodeError::InvalidChar(c, self.position))),
            };

            value = match value
                .checked_mul(3)
                .and_then(|v| v.checked_add(digit as u32))
            {
                Some(val) => val,
                None => return Some(Err(DollcodeError::InvalidInput)),
            };

            if value > 126 {
                return Some(Err(DollcodeError::InvalidInput));
            }

            self.position += 1;
        }

        if (32..=126).contains(&value) {
            Some(Ok(value as u8 as char))
        } else {
            Some(Err(DollcodeError::InvalidInput))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use heapless::String;

    #[test]
    fn test_ascii_roundtrip() {
        let test_cases = [
            (' ', 32, "▌▖▘"), // space
            ('!', 33, "▌▖▌"),
            ('"', 34, "▌▘▖"),
            ('#', 35, "▌▘▘"),
            ('$', 36, "▌▘▌"),
            ('%', 37, "▌▌▖"),
            ('&', 38, "▌▌▘"),
            ('\'', 39, "▌▌▌"),
            ('(', 40, "▖▖▖▖"),
            (')', 41, "▖▖▖▘"),
            ('*', 42, "▖▖▖▌"),
            ('+', 43, "▖▖▘▖"),
            (',', 44, "▖▖▘▘"),
            ('-', 45, "▖▖▘▌"),
            ('.', 46, "▖▖▌▖"),
            ('/', 47, "▖▖▌▘"),
            ('0', 48, "▖▖▌▌"),
            ('1', 49, "▖▘▖▖"),
            ('2', 50, "▖▘▖▘"),
            ('3', 51, "▖▘▖▌"),
            ('4', 52, "▖▘▘▖"),
            ('5', 53, "▖▘▘▘"),
            ('6', 54, "▖▘▘▌"),
            ('7', 55, "▖▘▌▖"),
            ('8', 56, "▖▘▌▘"),
            ('9', 57, "▖▘▌▌"),
            (':', 58, "▖▌▖▖"),
            (';', 59, "▖▌▖▘"),
            ('<', 60, "▖▌▖▌"),
            ('=', 61, "▖▌▘▖"),
            ('>', 62, "▖▌▘▘"),
            ('?', 63, "▖▌▘▌"),
            ('@', 64, "▖▌▌▖"),
            ('A', 65, "▖▌▌▘"),
            ('B', 66, "▖▌▌▌"),
            ('C', 67, "▘▖▖▖"),
            ('D', 68, "▘▖▖▘"),
            ('E', 69, "▘▖▖▌"),
            ('F', 70, "▘▖▘▖"),
            ('G', 71, "▘▖▘▘"),
            ('H', 72, "▘▖▘▌"),
            ('I', 73, "▘▖▌▖"),
            ('J', 74, "▘▖▌▘"),
            ('K', 75, "▘▖▌▌"),
            ('L', 76, "▘▘▖▖"),
            ('M', 77, "▘▘▖▘"),
            ('N', 78, "▘▘▖▌"),
            ('O', 79, "▘▘▘▖"),
            ('P', 80, "▘▘▘▘"),
            ('Q', 81, "▘▘▘▌"),
            ('R', 82, "▘▘▌▖"),
            ('S', 83, "▘▘▌▘"),
            ('T', 84, "▘▘▌▌"),
            ('U', 85, "▘▌▖▖"),
            ('V', 86, "▘▌▖▘"),
            ('W', 87, "▘▌▖▌"),
            ('X', 88, "▘▌▘▖"),
            ('Y', 89, "▘▌▘▘"),
            ('Z', 90, "▘▌▘▌"),
            ('[', 91, "▘▌▌▖"),
            ('\\', 92, "▘▌▌▘"),
            (']', 93, "▘▌▌▌"),
            ('^', 94, "▌▖▖▖"),
            ('_', 95, "▌▖▖▘"),
            ('`', 96, "▌▖▖▌"),
            ('a', 97, "▌▖▘▖"),
            ('b', 98, "▌▖▘▘"),
            ('c', 99, "▌▖▘▌"),
            ('d', 100, "▌▖▌▖"),
            ('e', 101, "▌▖▌▘"),
            ('f', 102, "▌▖▌▌"),
            ('g', 103, "▌▘▖▖"),
            ('h', 104, "▌▘▖▘"),
            ('i', 105, "▌▘▖▌"),
            ('j', 106, "▌▘▘▖"),
            ('k', 107, "▌▘▘▘"),
            ('l', 108, "▌▘▘▌"),
            ('m', 109, "▌▘▌▖"),
            ('n', 110, "▌▘▌▘"),
            ('o', 111, "▌▘▌▌"),
            ('p', 112, "▌▌▖▖"),
            ('q', 113, "▌▌▖▘"),
            ('r', 114, "▌▌▖▌"),
            ('s', 115, "▌▌▘▖"),
            ('t', 116, "▌▌▘▘"),
            ('u', 117, "▌▌▘▌"),
            ('v', 118, "▌▌▌▖"),
            ('w', 119, "▌▌▌▘"),
            ('x', 120, "▌▌▌▌"),
            ('y', 121, "▖▖▖▖▖"),
            ('z', 122, "▖▖▖▖▘"),
            ('{', 123, "▖▖▖▖▌"),
            ('|', 124, "▖▖▖▘▖"),
            ('}', 125, "▖▖▖▘▘"),
            ('~', 126, "▖▖▖▘▌"),
        ];

        for &(c, _, encoded) in &test_cases {
            // Decode test
            let mut decoder = TextDecoder::new(encoded);
            let decoded = decoder.next().unwrap().unwrap();

            assert_eq!(decoded, c, "Decoded character should match original");
        }
    }

    #[test]
    fn test_invalid_input() {
        // Test invalid symbol
        let invalid_input = "▖▌X";
        let mut decoder = TextDecoder::new(invalid_input);
        match decoder.next() {
            Some(Err(DollcodeError::InvalidChar(c, pos))) => {
                assert_eq!(c, 'X');
                assert_eq!(pos, 2);
            }
            _ => panic!("Expected InvalidChar error"),
        }

        // Test value exceeding ASCII range
        let invalid_input = "▖▖▖▌▘";
        let mut decoder = TextDecoder::new(invalid_input);
        match decoder.next() {
            Some(Err(DollcodeError::InvalidInput)) => (),
            _ => panic!("Expected InvalidInput error"),
        }

        // Test incomplete sequence
        let invalid_input = "▖▌";
        let mut decoder = TextDecoder::new(invalid_input);
        match decoder.next() {
            Some(Err(DollcodeError::InvalidInput)) => (),
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_encoding_with_delimiter() {
        let text = "Hi!";
        let mut encoded = heapless::Vec::<char, 128>::new();

        for segment in TextIterator::new(text) {
            let segment = segment.unwrap();
            encoded.extend_from_slice(segment.as_chars()).unwrap();
        }

        let expected = "▘▖▘▌\u{200d}▌▘▖▌\u{200d}▌▖▌\u{200d}";
        let encoded_str: String<128> = encoded.iter().collect();
        assert_eq!(
            encoded_str, expected,
            "Encoded string does not match expected value"
        );
    }

    #[test]
    fn test_roundtrip_with_delimiter() {
        let original = "Hello, World!";
        let mut encoded = heapless::Vec::<char, 256>::new();

        for segment in TextIterator::new(original) {
            let segment = segment.unwrap();
            encoded.extend_from_slice(segment.as_chars()).unwrap();
        }

        let encoded_str: String<256> = encoded.iter().collect();
        let decoder = TextDecoder::new(&encoded_str);
        let mut decoded = String::<256>::new();

        for result in decoder {
            let c = result.unwrap();
            decoded.push(c).unwrap();
        }

        assert_eq!(decoded, original, "Roundtrip encoding/decoding failed");
    }
}
