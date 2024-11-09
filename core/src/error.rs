//! Error types for dollcode operations.

use core::fmt;
use owo_colors::OwoColorize;

/// Errors that can occur during dollcode operations
#[derive(Debug)]
pub enum DollcodeError {
    /// Input validation failed
    InvalidInput,
    /// Invalid character for text encoding
    InvalidChar(char, usize),
    /// Value overflow occurred
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

/// Result type from core
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
