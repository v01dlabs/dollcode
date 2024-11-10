//! Error types for dollcode operations.
//!
//! This module provides error handling for the dollcode crate.
//! All error types and operations avoid heap allocations to maintain #[no_std]
//! compatibility.
//!
//! # Examples
//!
//! ```rust
//! #[test]
//! fn test_invalid_decode() {
//!    let result = dollcode::from_dollcode(&['A', 'B', 'C']);
//!    assert!(matches!(result, Err(DollcodeError::InvalidInput)));
//!}
//!```

use core::fmt;
use owo_colors::OwoColorize;

/// Errors that can occur during dollcode operations
#[derive(Debug)]
pub enum DollcodeError {
    /// Input validation failed due to invalid characters or sequence
    ///
    /// This error occurs when:
    /// - Input contains characters that aren't valid dollcode (▖,▘,▌)
    /// - Text segment has incorrect length
    /// - Invalid sequence structure
    InvalidInput,

    /// Invalid character encountered during text encoding
    ///
    /// Contains the invalid character and its position in the input.
    /// This error occurs when attempting to encode characters that
    /// aren't in the supported ASCII set.
    InvalidChar(char, usize),

    /// Value overflow occurred during encoding or decoding
    ///
    /// This error occurs when:
    /// - Encoding a number that's too large
    /// - Decoding a sequence that would overflow u64
    /// - Text segment position overflow
    Overflow,
}

impl fmt::Display for DollcodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput => write!(f, "{}", "Invalid dollcode sequence".purple()),
            Self::InvalidChar(c, pos) => {
                write!(f, "{}", "Invalid character".purple())?;
                write!(f, ": '{}'", c)?;
                write!(f, " at position {}", pos)
            }
            Self::Overflow => write!(f, "{}", "Value overflow".red()),
        }
    }
}

/// Result type specialized for dollcode operations
pub type Result<T> = core::result::Result<T, DollcodeError>;

#[cfg(test)]
mod tests {
    use super::*;
    use core::fmt::Write;
    use heapless::String;

    #[test]
    fn test_error_messages() {
        // Test invalid input
        let mut s: String<64> = String::new();
        let _ = write!(s, "{}", DollcodeError::InvalidInput);
        assert!(s.contains("Invalid dollcode sequence"));

        // Test invalid char with position
        s.clear();
        let _ = write!(s, "{}", DollcodeError::InvalidChar('☺', 5));
        assert!(s.contains("Invalid character"));
        assert!(s.contains('☺'));
        assert!(s.contains('5'));

        // Test overflow
        s.clear();
        let _ = write!(s, "{}", DollcodeError::Overflow);
        assert!(s.contains("Value overflow"));
    }

    #[test]
    fn test_error_display() {
        // Test display implementation doesn't allocate
        let err = DollcodeError::InvalidChar('!', 0);
        let mut s: String<64> = String::new();
        let _ = write!(s, "{}", err);
        assert!(!s.is_empty());
    }
}
