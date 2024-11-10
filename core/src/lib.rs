#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs, missing_debug_implementations)]
#![warn(rust_2018_idioms, unreachable_pub)]
//! # dollcode
//!
//! A zero-allocation trinary encoding system that represents numbers and text using box-drawing
//! characters (▖, ▘, ▌).
//!
//! ## Features
//!
//! - **Zero Allocation**: All operations use fixed-size buffers and avoid heap allocations
//! - **#[no_std] Compatible**: Can be used in embedded applications
//! - **Text Encoding**: Convert ASCII text to dollcode sequences
//! - **Numeric Encoding**: Convert numbers to base-3 dollcode representation
//! - **Predictable Memory**: Fixed memory usage regardless of input size
//!
//! ## Quick Start
//!
//! ```rust
//! # use dollcode::{to_dollcode, from_dollcode, Result};
//! # fn main() -> Result<()> {
//! // Encode a number
//! let encoded = to_dollcode(42)?;
//! assert_eq!(encoded.to_string(), "▖▖▖▌");
//!
//! // Decode back to number
//! let decoded = from_dollcode(encoded.as_chars())?;
//! assert_eq!(decoded, 42);
//! # Ok(())
//! # }
//! ```
//!
//!
//! ## Text Encoding Example
//!
//! ```rust
//! # use dollcode::text::{TextIterator, TextDecoder};
//! # fn main() -> dollcode::Result<()> {
//! let text = "Hi!";
//! let mut encoded = heapless::Vec::<char, 128>::new();
//!
//! for segment in TextIterator::new(text) {
//!     let segment = segment?;
//!     assert_eq!(segment.as_chars().len(), 5);
//!     encoded.extend_from_slice(segment.as_chars()).unwrap(); // Use unwrap instead of ? for Vec result
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Numeric Encoding Details
//!
//! Numbers are encoded in base-3 using the mapping:
//! - ▖ = 1
//! - ▘ = 2
//! - ▌ = 3
//!
//! For example:
//! ```text
//! 1  → ▖      (1)
//! 2  → ▘      (2)
//! 3  → ▌      (3)
//! 4  → ▖▖     (1×3 + 1)
//! 13 → ▖▖▖    (1×9 + 1×3 + 1)
//! 42 → ▖▖▖▌   (1×27 + 1×9 + 1×3 + 3)
//! ```
//!
//! ## Error Handling
//!
//! All operations return a [`Result`] type that can contain the following errors:
//! - [`DollcodeError::InvalidInput`]: Input validation failed
//! - [`DollcodeError::InvalidChar`]: Invalid character for text encoding
//! - [`DollcodeError::Overflow`]: Value overflow occurred
//!
//! ## Zero Allocation Guarantee
//!
//! This crate makes zero heap allocations by using fixed-size buffers from the [`heapless`] crate.
//! All operations work with stack memory only.
//!
//! ## Performance
//!
//! - Encoding/decoding operations are O(1) for numbers
//! - Text operations are O(n) where n is the input length
//! - No heap allocations or system calls
//! - Constant memory usage regardless of input size
//!
//! ## Examples
//!
//! More examples can be found in the documentation for individual functions.

pub mod error;
pub mod text;

pub use error::{DollcodeError, Result};

/// Maximum length of a dollcode sequence
pub const MAX_DOLLCODE_SIZE: usize = 64;

/// The three characters used in dollcode representation in value order.
/// Maps 1->▖, 2->▘, 3->▌
pub const CHAR_MAP: [char; 3] = ['▖', '▘', '▌'];

/// A fixed-size dollcode sequence with zero heap allocation
#[derive(Debug, Clone, Copy)]
pub struct Dollcode {
    chars: [char; MAX_DOLLCODE_SIZE],
    len: usize,
}

impl Default for Dollcode {
    fn default() -> Self {
        Self::new()
    }
}

impl Dollcode {
    /// Creates an empty dollcode sequence
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use dollcode::Dollcode;
    /// let dollcode = Dollcode::new();
    /// assert!(dollcode.is_empty());
    /// assert_eq!(dollcode.len(), 0);
    /// ```
    #[inline]
    pub fn new() -> Self {
        Self {
            chars: ['\0'; MAX_DOLLCODE_SIZE],
            len: 0,
        }
    }

    /// Returns a slice of the valid characters in this sequence
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use dollcode::{to_dollcode, Result};
    /// # fn main() -> Result<()> {
    /// let dollcode = to_dollcode(42)?;
    /// assert_eq!(dollcode.as_chars(), &['▖', '▖', '▖', '▌']);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn as_chars(&self) -> &[char] {
        &self.chars[..self.len]
    }

    /// Returns the number of characters in this sequence
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use dollcode::{to_dollcode, Result};
    /// # fn main() -> Result<()> {
    /// let dollcode = to_dollcode(42)?;
    /// assert_eq!(dollcode.len(), 4);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns true if this sequence is empty
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use dollcode::{to_dollcode, Dollcode, Result};
    /// # fn main() -> Result<()> {
    /// let empty = Dollcode::new();
    /// assert!(empty.is_empty());
    ///
    /// let dollcode = to_dollcode(42)?;
    /// assert!(!dollcode.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// Display implementation for Dollcode that renders the sequence as a string of box-drawing characters.
///
/// # Examples
///
/// ```rust
/// # use dollcode::{to_dollcode, Result};
/// # fn main() -> Result<()> {
/// let dollcode = to_dollcode(42)?;
/// assert_eq!(dollcode.to_string(), "▖▖▖▌");
/// # Ok(())
/// # }
/// ```
///
/// # Notes
///
/// - Only includes the valid characters in the sequence
/// - Empty sequences display as an empty string
/// - No separators or additional formatting are added
impl core::fmt::Display for Dollcode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for &c in self.as_chars() {
            write!(f, "{}", c)?;
        }
        Ok(())
    }
}

/// Encodes a number into dollcode using base-3.
/// Each digit represents a value 1-3, mapped to ▖,▘,▌ respectively.
///
/// # Examples
///
/// ```rust
/// # use dollcode::{to_dollcode, Result};
/// # fn main() -> Result<()> {
/// let dollcode = to_dollcode(42)?;
/// assert_eq!(dollcode.as_chars(), &['▖', '▖', '▖', '▌']);
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns [`DollcodeError::Overflow`] if the number is too large to encode.
pub fn to_dollcode(mut num: u64) -> Result<Dollcode> {
    if num == 0 {
        return Ok(Dollcode::new());
    }

    let mut dollcode = Dollcode::new();
    let mut output = [0u8; MAX_DOLLCODE_SIZE]; // Stack-allocated buffer
    let mut digits = 0;

    // Convert to base-3 with digits representing values 1-3
    while num > 0 {
        if digits >= MAX_DOLLCODE_SIZE {
            return Err(DollcodeError::Overflow);
        }

        let rem = (num - 1) % 3; // Get 0-2 remainder
        output[digits] = rem as u8; // Store remainder directly
        num = (num - 1 - rem) / 3; // Reduce number
        digits += 1;
    }

    // Map remainders to characters in reverse order
    dollcode.len = digits;
    for i in 0..digits {
        dollcode.chars[i] = CHAR_MAP[output[digits - 1 - i] as usize];
    }

    Ok(dollcode)
}

/// Decodes dollcode back to a number.
/// Interprets the sequence as base-3 where:
/// ▖=1, ▘=2, ▌=3
///
/// # Examples
///
/// ```rust
/// # use dollcode::{from_dollcode, Result};
/// # fn main() -> Result<()> {
/// let chars = ['▖', '▖', '▖', '▌'];
/// let num = from_dollcode(&chars)?;
/// assert_eq!(num, 42);
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns:
/// - [`DollcodeError::InvalidInput`] if the sequence contains invalid characters
/// - [`DollcodeError::Overflow`] if the decoded value would overflow u64
pub fn from_dollcode(chars: &[char]) -> Result<u64> {
    if chars.is_empty() {
        return Ok(0);
    }

    let mut result = 0u64;

    // Process each character, building up the number
    for &c in chars {
        // Multiply by base
        result = result.checked_mul(3).ok_or(DollcodeError::Overflow)?;

        // Map character to value and add
        let val = match c {
            '▖' => 1, // Maps to 1
            '▘' => 2, // Maps to 2
            '▌' => 3, // Maps to 3
            _ => return Err(DollcodeError::InvalidInput),
        };

        result = result.checked_add(val).ok_or(DollcodeError::Overflow)?;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use heapless::String;

    #[test]
    fn test_encoding_sequence() {
        let cases = [
            (0, ""),
            (1, "▖"),    // 1
            (2, "▘"),    // 2
            (3, "▌"),    // 3
            (4, "▖▖"),   // 1×3 + 1
            (5, "▖▘"),   // 1×3 + 2
            (6, "▖▌"),   // 1×3 + 3
            (7, "▘▖"),   // 2×3 + 1
            (8, "▘▘"),   // 2×3 + 2
            (9, "▘▌"),   // 2×3 + 3
            (10, "▌▖"),  // 3×3 + 1
            (11, "▌▘"),  // 3×3 + 2
            (12, "▌▌"),  // 3×3 + 3
            (13, "▖▖▖"), // 1×9 + 1×3 + 1
            // Start at 0
            //     '▖' -> 1:  0 * 3 + 1 = 1
            //     '▖' -> 1:  1 * 3 + 1 = 4
            //     '▖' -> 1:  4 * 3 + 1 = 13
            //     '▌' -> 3: 13 * 3 + 3 = 42
            (42, "▖▖▖▌"), // 1×27 + 1×9 + 1×3 + 3
        ];

        for &(num, expected) in &cases {
            let encoded = to_dollcode(num).unwrap();
            let encoded_str: String<64> = encoded.as_chars().iter().collect();
            assert_eq!(
                encoded_str, expected,
                "Encoding {} failed - got {}, expected {}",
                num, encoded_str, expected
            );

            if !expected.is_empty() {
                let decoded = from_dollcode(&encoded.chars[..encoded.len]).unwrap();
                assert_eq!(
                    decoded, num,
                    "Decoding {} failed - got {}, expected {}",
                    encoded_str, decoded, num
                );
            }
        }
    }

    #[test]
    fn test_decoding_sequence() {
        let cases = [
            ("", 0),
            ("▖", 1),
            ("▘", 2),
            ("▌", 3),
            ("▖▖", 4),
            ("▖▘", 5),
            ("▖▌", 6),
            ("▘▖", 7),
            ("▘▘", 8),
            ("▘▌", 9),
            ("▌▖", 10),
            ("▌▘", 11),
            ("▌▌", 12),
            ("▖▖▖", 13),
            ("▖▖▖▌", 42),
        ];

        for &(input, expected) in &cases {
            let chars: heapless::Vec<char, 64> = input.chars().collect();
            let decoded = from_dollcode(&chars).unwrap();
            assert_eq!(
                decoded, expected,
                "Decoding {} failed - got {}, expected {}",
                input, decoded, expected
            );
        }
    }

    #[test]
    fn test_edge_cases() {
        // Test overflow handling
        let buffer = [CHAR_MAP[0]; MAX_DOLLCODE_SIZE + 1];
        assert!(from_dollcode(&buffer).is_err());

        // Test invalid characters
        assert!(from_dollcode(&['a', 'b', 'c']).is_err());

        // Test zero
        let encoded = to_dollcode(0).unwrap();
        assert!(encoded.is_empty());
        assert_eq!(from_dollcode(&[]).unwrap(), 0);
    }

    #[test]
    fn test_large_numbers() {
        let large_cases = [
            1000,
            10_000,
            100_000,
            1_000_000,
            440729,
        ];

        for &num in &large_cases {
            let encoded = to_dollcode(num).unwrap();
            let decoded = from_dollcode(encoded.as_chars()).unwrap();
            assert_eq!(
                decoded, num,
                "Round-trip failed for {} - got {}",
                num, decoded
            );
        }
    }

    #[test]
    fn test_base3_properties() {
        let test_nums = [4, 13, 42, 100];

        for &num in &test_nums {
            let encoded = to_dollcode(num).unwrap();

            // Verify base-3 interpretation
            let mut value = 0u64;
            for &c in encoded.as_chars() {
                value *= 3;
                value += match c {
                    '▖' => 1,
                    '▘' => 2,
                    '▌' => 3,
                    _ => panic!("Invalid dollcode char"),
                };
            }
            assert_eq!(value, num, "Base-3 check failed for {}", num);
        }
    }
}
