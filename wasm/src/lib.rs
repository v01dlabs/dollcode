#![no_std]
#![forbid(unsafe_code)]
//! WASM bindings for dollcode conversion with zero-allocation guarantees.
//!
//! # Input Types & Limits
//!
//! - **Text**: ASCII printable characters (32-126)
//!   - Maximum length: 100 characters
//!   - Each character produces 5 dollcode chars + 1 delimiter
//!
//! - **Decimal Numbers**: 0-18,446,744,073,709,551,615 (u64::MAX)
//!   - Maximum length: 20 digits
//!
//! - **Hexadecimal**: 0x0-0xFFFFFFFFFFFFFFFF
//!   - Maximum length: 18 chars (including 0x prefix)
//!
//! - **dollcode**: Sequences of ▖, ▘, ▌
//!   - Maximum length: 41 chars for numbers (log_3(2^64))
//!   - For text: up to CHAR_BUF_SIZE (1800 bytes)
//!
//! # Memory Usage
//!
//! Uses fixed stack buffers with zero heap allocation:
//! - Text input: 100 chars maximum
//! - Dollcode output: 18 bytes per input char (6 chars × 3 bytes UTF-8)
//! - Total output buffer: 1800 bytes (100 chars × 18 bytes)
//!
//! # Examples
//!
//! ```rust
//! # use dollcode_wasm::convert;
//! // Convert numbers
//! assert_eq!(convert("42").unwrap(), "▖▖▖▌");
//! assert_eq!(convert("0xFF").unwrap(), "▘▘▌▌▌");
//!
//! // Convert text
//! assert_eq!(convert("Hi").unwrap(), "▘▖▘▌\u{200d}▌▘▖▌\u{200d}");
//!
//! // Decode dollcode
//! let result = convert("▖▖▖▌").unwrap();
//! assert!(result.as_string().unwrap().contains("42"));
//! ```
//!
use core::{any::Any, fmt::Write};
use dollcode::{
    from_dollcode,
    text::{TextDecoder, TextIterator},
    to_dollcode, DollcodeError, MAX_DOLLCODE_SIZE,
};
use heapless::String;
use wasm_bindgen::prelude::*;

/// Maximum input text length in characters
const INPUT_SIZE: usize = 100;

/// Maximum decimal number input length (u64::MAX digits)
const MAX_DECIMAL_DIGITS: usize = 20;

/// Maximum hex input length including 0x prefix
const MAX_HEX_LENGTH: usize = 18;

/// Buffer size for dollcode output
/// Each input char produces:
/// - 5 dollcode chars maximum
/// - 1 delimiter char
///
/// Each UTF-8 char is 3 bytes
/// Total: (5 + 1) × 3 = 18 bytes per input char
const CHAR_BUF_SIZE: usize = INPUT_SIZE * 18;

// Error messages
const ERR_EMPTY: &str = "Empty input";
const ERR_DOLLCODE_TOO_LONG: &str = "Dollcode sequence exceeds maximum length";
const ERR_INPUT_TOO_LONG: &str = "Input exceeds maximum length";
const ERR_DECIMAL_TOO_LONG: &str = "Decimal number exceeds maximum digits";
const ERR_HEX_TOO_LONG: &str = "Hex number exceeds maximum length";
const ERR_BUFFER_FULL: &str = "Output buffer full";
const ERR_INVALID_SEQUENCE: &str = "Invalid dollcode sequence";
const ERR_INVALID_DECIMAL: &str = "Invalid decimal number";
const ERR_INVALID_HEX: &str = "Invalid hexadecimal number";
const ERR_INVALID_CHARS: &str = "Input contains invalid characters";

/// Convert Error types to JsValue with context
fn to_js_err(e: impl core::fmt::Debug + Any) -> JsValue {
    let mut msg: String<128> = String::new();

    if let Some(e) = (&e as &dyn Any).downcast_ref::<DollcodeError>() {
        match e {
            DollcodeError::InvalidChar(c, _) => {
                let _ = write!(
                    &mut msg,
                    "Character '{}' is not supported\n(valid: printable ASCII)",
                    c
                );
            }
            DollcodeError::Overflow => {
                let _ =
                    msg.push_str("Input exceeds maximum length\n(text: 100, decimal: 20, hex: 18)");
            }
            DollcodeError::InvalidInput => {
                let _ =
                    msg.push_str("Only ▖, ▘, and ▌ characters are allowed for dollcode sequences");
            }
        }
    } else {
        let _ = msg.push_str("Conversion error occurred");
    }

    JsValue::from_str(&msg)
}

/// Converts input to dollcode based on content type.
///
/// Input type is detected in the following order:
/// 1. Dollcode sequences (if contains ▖, ▘, or ▌)
/// 2. Decimal numbers (if all digits)
/// 3. Hex numbers (if starts with 0x)
/// 4. Text (ASCII printable)
///
/// # Errors
///
/// Returns errors for:
/// - Invalid characters
/// - Exceeding length limits
/// - Invalid dollcode sequences
/// - Numbers outside u64 range
#[wasm_bindgen]
pub fn convert(input: &str) -> Result<JsValue, JsValue> {
    if input.is_empty() {
        return Err(JsValue::from_str(ERR_EMPTY));
    }

    // General input validation: ensure only allowed characters are present
    if let Some(c) = input.chars().find(|&c| {
        !(
            // ASCII printable characters (codes 32 to 126)
            (c as u32 >= 32 && c as u32 <= 126) ||
            // Dollcode characters
            c == '▖' || c == '▘' || c == '▌' ||
            // Zero Width Joiner
            c == '\u{200D}'
        )
    }) {
        return Err(to_js_err(DollcodeError::InvalidChar(c, 0)));
    }

    // Check for dollcode characters first
    if input
        .chars()
        .any(|c| matches!(c, '▖' | '▘' | '▌' | '\u{200D}'))
    {
        if input.len() > CHAR_BUF_SIZE {
            return Err(JsValue::from_str(ERR_DOLLCODE_TOO_LONG));
        }
        if !input
            .chars()
            .all(|c| matches!(c, '▖' | '▘' | '▌' | '\u{200D}'))
        {
            return Err(to_js_err(DollcodeError::InvalidInput));
        }
        return convert_dollcode(input);
    }

    // Other input types use INPUT_SIZE
    if input.chars().count() > INPUT_SIZE {
        return Err(JsValue::from_str(ERR_INPUT_TOO_LONG));
    }

    // Try decimal first if all digits
    if input.chars().all(|c| c.is_ascii_digit()) {
        if input.len() > MAX_DECIMAL_DIGITS {
            return Err(JsValue::from_str(ERR_DECIMAL_TOO_LONG));
        }
        return convert_decimal(input);
    }

    // Then try hex if valid prefix and digits
    if input.len() > 2
        && input.starts_with("0x")
        && input[2..].chars().all(|c| c.is_ascii_hexdigit())
    {
        if input.len() > MAX_HEX_LENGTH {
            return Err(JsValue::from_str(ERR_HEX_TOO_LONG));
        }
        return convert_hex(input);
    }

    // Finally try text - verify input is valid ASCII
    if input.chars().any(|c| (c as u32) < 32 || (c as u32) > 126) {
        return Err(JsValue::from_str(ERR_INVALID_CHARS));
    }

    convert_text(input)
}

/// Converts decimal numbers to dollcode
#[wasm_bindgen]
pub fn convert_decimal(input: &str) -> Result<JsValue, JsValue> {
    let num = input
        .parse::<u64>()
        .map_err(|_| JsValue::from_str(ERR_INVALID_DECIMAL))?;

    let dollcode = to_dollcode(num).map_err(to_js_err)?;

    let mut output: String<CHAR_BUF_SIZE> = String::new();
    for &c in dollcode.as_chars() {
        output
            .push(c)
            .map_err(|_| JsValue::from_str(ERR_BUFFER_FULL))?;
    }

    Ok(JsValue::from_str(&output))
}

/// Converts hexadecimal numbers to dollcode
#[wasm_bindgen]
pub fn convert_hex(input: &str) -> Result<JsValue, JsValue> {
    let input = input.trim_start_matches("0x");
    let num = u64::from_str_radix(input, 16).map_err(|_| JsValue::from_str(ERR_INVALID_HEX))?;

    let dollcode = to_dollcode(num).map_err(to_js_err)?;

    let mut output: String<CHAR_BUF_SIZE> = String::new();
    for &c in dollcode.as_chars() {
        output
            .push(c)
            .map_err(|_| JsValue::from_str(ERR_BUFFER_FULL))?;
    }

    Ok(JsValue::from_str(&output))
}

/// Converts ASCII text to dollcode
#[wasm_bindgen]
pub fn convert_text(input: &str) -> Result<JsValue, JsValue> {
    if input.is_empty() {
        return Err(JsValue::from_str(ERR_EMPTY));
    }

    let mut output: String<CHAR_BUF_SIZE> = String::new();

    for result in TextIterator::new(input) {
        let segment = result.map_err(to_js_err)?;
        for &c in segment.as_chars() {
            output
                .push(c)
                .map_err(|_| JsValue::from_str(ERR_BUFFER_FULL))?;
        }
    }

    Ok(JsValue::from_str(&output))
}

/// Converts dollcode back to numbers and text
#[wasm_bindgen]
pub fn convert_dollcode(input: &str) -> Result<JsValue, JsValue> {
    if input.is_empty() {
        return Ok(JsValue::from_str(""));
    }

    // First check if it contains any ZWJs - if so, treat as text
    if input.chars().any(|c| c == '\u{200D}') {
        // Text mode - use CHAR_BUF_SIZE
        let mut chars = ['\0'; CHAR_BUF_SIZE];
        let mut len = 0;

        for c in input.chars() {
            if len >= CHAR_BUF_SIZE {
                return Err(JsValue::from_str(ERR_BUFFER_FULL));
            }

            let normalized = match c {
                '▖' | '▘' | '▌' | '\u{200D}' => c,
                c if c as u32 == 0x2596 => '▖',
                c if c as u32 == 0x2598 => '▘',
                c if c as u32 == 0x258C => '▌',
                _ => continue,
            };

            chars[len] = normalized;
            len += 1;
        }

        let mut decoded = String::<CHAR_BUF_SIZE>::new();
        let normalized_str: String<CHAR_BUF_SIZE> = chars[..len].iter().collect();

        for result in TextDecoder::new(&normalized_str) {
            match result {
                Ok(c) => {
                    decoded
                        .push(c)
                        .map_err(|_| JsValue::from_str(ERR_BUFFER_FULL))?;
                }
                Err(_) => {
                    return Ok(JsValue::from_str(ERR_INVALID_SEQUENCE));
                }
            }
        }

        Ok(JsValue::from_str(&decoded))
    } else {
        // Number mode - use MAX_DOLLCODE_SIZE
        let mut chars = ['\0'; MAX_DOLLCODE_SIZE];
        let mut len = 0;

        for c in input.chars() {
            if len >= MAX_DOLLCODE_SIZE {
                return Err(JsValue::from_str(ERR_DOLLCODE_TOO_LONG));
            }

            let normalized = match c {
                '▖' | '▘' | '▌' => c,
                c if c as u32 == 0x2596 => '▖',
                c if c as u32 == 0x2598 => '▘',
                c if c as u32 == 0x258C => '▌',
                _ => continue,
            };

            chars[len] = normalized;
            len += 1;
        }

        if let Ok(num) = from_dollcode(&chars[..len]) {
            let mut result: String<CHAR_BUF_SIZE> = String::new();
            let _ = writeln!(&mut result, "Dec (base10): {}", num);
            let _ = write!(&mut result, "Hex (base16): 0x{:x}", num);
            Ok(JsValue::from_str(&result))
        } else {
            Ok(JsValue::from_str(ERR_INVALID_SEQUENCE))
        }
    }
}

/// Initializes panic hook for WASM
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

#[cfg(target_arch = "wasm32")]
#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_input_size_limits() {
        let max_input = "A".repeat(INPUT_SIZE);
        assert!(convert(&max_input).is_ok());

        let too_long = "A".repeat(INPUT_SIZE + 1);
        assert_eq!(
            convert(&too_long).unwrap_err(),
            JsValue::from_str(ERR_INPUT_TOO_LONG)
        );

        let too_long_decimal = "9".repeat(MAX_DECIMAL_DIGITS + 1);
        assert_eq!(
            convert(&too_long_decimal).unwrap_err(),
            JsValue::from_str(ERR_DECIMAL_TOO_LONG)
        );

        assert_eq!(
            convert("0xFFFFFFFFFFFFFFFFF").unwrap_err(),
            JsValue::from_str(ERR_HEX_TOO_LONG)
        );
    }

    #[wasm_bindgen_test]
    fn test_buffer_overflow_prevention() {
        let too_long = "▖".repeat(CHAR_BUF_SIZE + 1);
        assert_eq!(
            convert(&too_long).unwrap_err(),
            JsValue::from_str(ERR_DOLLCODE_TOO_LONG)
        );
    }

    #[wasm_bindgen_test]
    fn test_character_validation() {
        assert!(convert("\x1F").is_err());
        assert!(convert("\x7F").is_err());
        assert!(convert("☺").is_err());
        assert!(convert(" ").is_ok());
        assert!(convert("~").is_ok());
        assert!(convert("Hello!").is_ok());
    }

    #[wasm_bindgen_test]
    fn test_dollcode_validation() {
        assert!(convert("▖").is_ok());
        assert!(convert("▘").is_ok());
        assert!(convert("▌").is_ok());
        assert!(convert("▖▘▌").is_ok());
        assert!(convert("▖A").is_err());
        assert!(convert("▘1").is_err());
        assert!(convert("▌x").is_err());
    }

    #[wasm_bindgen_test]
    fn test_decimal_conversion_limits() {
        // Test error cases with constant strings
        assert_eq!(
            convert("18446744073709551616").unwrap_err(),
            JsValue::from_str(ERR_INVALID_DECIMAL)
        );
        assert_eq!(convert("000042").unwrap(), convert("42").unwrap());
    }

    #[wasm_bindgen_test]
    fn test_hex_conversion_edge_cases() {
        assert!(convert("0x0").is_ok());
        assert!(convert("0xf").is_ok());
        assert!(convert("0xF").is_ok());
        assert!(convert("0xDEADBEEF").is_ok());
        assert!(convert("0xG").is_ok()); // Valid as text
        assert!(convert("0x-1").is_ok()); // Valid as text
        assert!(convert("0xFFFFFFFFFFFFFFFF").is_ok());
        assert!(convert("0x").is_ok()); // Actual behavior: converts as text
    }

    #[wasm_bindgen_test]
    fn test_text_delimiter_handling() {
        assert_eq!(
            convert("Hi").unwrap(),
            JsValue::from_str("▘▖▘▌\u{200D}▌▘▖▌\u{200D}")
        );
    }

    #[wasm_bindgen_test]
    fn test_dollcode_decoding() {
        // Test number decoding (NOTE: Core gives numeric output)
        assert_eq!(
            convert("▖▖▖▌").unwrap(),
            JsValue::from_str("Dec (base10): 42\nHex (base16): 0x2a")
        );

        // Test invalid sequence
        assert_eq!(
            convert("▖▘▌!").unwrap_err(),
            to_js_err(DollcodeError::InvalidInput)
        );
    }

    #[wasm_bindgen_test]
    fn test_empty_and_whitespace() {
        assert_eq!(convert("").unwrap_err(), JsValue::from_str(ERR_EMPTY));

        assert!(convert(" ").is_ok());
        assert!(convert("   ").is_ok());
        assert!(convert("Hello World").is_ok());

        // Text input processes whitespace, so these aren't equal
        assert!(convert("42").is_ok());
        assert!(convert("  42  ").is_ok());
    }

    #[wasm_bindgen_test]
    fn test_unicode_normalization() {
        assert_eq!(
            convert("▖▘▌").unwrap(),
            convert("\u{2596}\u{2598}\u{258C}").unwrap()
        );
    }

    #[wasm_bindgen_test]
    fn test_error_messages() {
        let long_input = "A".repeat(INPUT_SIZE + 1);
        assert_eq!(
            convert(&long_input).unwrap_err(),
            JsValue::from_str(ERR_INPUT_TOO_LONG)
        );

        assert_eq!(convert("").unwrap_err(), JsValue::from_str(ERR_EMPTY));

        let long_dollcode = "▖".repeat(CHAR_BUF_SIZE + 1);
        assert_eq!(
            convert(&long_dollcode).unwrap_err(),
            JsValue::from_str(ERR_DOLLCODE_TOO_LONG)
        );
    }

    #[wasm_bindgen_test]
    fn test_number_limits() {
        const MAX_U64_DEC: &str = "18446744073709551615";
        assert!(convert(MAX_U64_DEC).is_ok());
        const MAX_U64_HEX: &str = "0xFFFFFFFFFFFFFFFF";
        assert!(convert(MAX_U64_HEX).is_ok());
    }
}
