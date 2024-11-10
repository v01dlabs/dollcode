#![no_std]
#![forbid(unsafe_code)]

use core::fmt::Write;
use dollcode::{
    from_dollcode,
    text::{TextDecoder, TextIterator, SEGMENT_SIZE},
    to_dollcode,
};
use heapless::{String, Vec};
use wasm_bindgen::prelude::*;

const OUTPUT_SIZE: usize = 1024;
const CHAR_BUF_SIZE: usize = 2048;

// Simple error conversion with context
fn to_js_err(e: impl core::fmt::Debug) -> JsValue {
    let msg = if let Some(type_name) = core::any::type_name_of_val(&e).split("::").last() {
        match type_name {
            "Overflow" => "Input limit exceeded",
            "InvalidInput" => "Invalid dollcode sequence",
            "InvalidChar" => "Invalid character detected",
            _ => "Conversion error occurred",
        }
    } else {
        "Conversion error occurred"
    };
    JsValue::from_str(msg)
}

#[wasm_bindgen]
pub fn convert_decimal(input: &str) -> Result<JsValue, JsValue> {
    // Handle empty input gracefully
    if input.is_empty() {
        return Ok(JsValue::from_str(""));
    }

    let num = input.parse::<u64>().map_err(to_js_err)?;
    let dollcode = to_dollcode(num).map_err(to_js_err)?;

    let mut output: String<OUTPUT_SIZE> = String::new();
    for &c in dollcode.as_chars() {
        output.push(c).map_err(to_js_err)?;
    }

    Ok(JsValue::from_str(&output))
}

#[wasm_bindgen]
pub fn convert_hex(input: &str) -> Result<JsValue, JsValue> {
    // Handle empty input gracefully
    if input.is_empty() {
        return Ok(JsValue::from_str(""));
    }

    let input = input.trim_start_matches("0x");
    let num = u64::from_str_radix(input, 16).map_err(to_js_err)?;
    let dollcode = to_dollcode(num).map_err(to_js_err)?;

    let mut output: String<OUTPUT_SIZE> = String::new();
    for &c in dollcode.as_chars() {
        output.push(c).map_err(to_js_err)?;
    }

    Ok(JsValue::from_str(&output))
}

#[wasm_bindgen]
pub fn convert_text(input: &str) -> Result<JsValue, JsValue> {
    // Handle empty input gracefully
    if input.is_empty() {
        return Ok(JsValue::from_str(""));
    }

    let mut output: String<OUTPUT_SIZE> = String::new();

    for segment_result in TextIterator::new(input) {
        let segment = segment_result.map_err(to_js_err)?;
        for &c in segment.as_chars() {
            output.push(c).map_err(to_js_err)?;
        }
    }

    Ok(JsValue::from_str(&output))
}

#[wasm_bindgen]
pub fn convert_dollcode(input: &str) -> Result<JsValue, JsValue> {
    // Handle empty input gracefully
    if input.is_empty() {
        return Ok(JsValue::from_str(""));
    }

    let mut chars: Vec<char, CHAR_BUF_SIZE> = Vec::new();
    for c in input.chars() {
        chars.push(c).map_err(to_js_err)?;
    }

    // If it looks like encoded text (multiple of 5 chars)
    if chars.len() % SEGMENT_SIZE == 0 {
        let mut decoded: String<OUTPUT_SIZE> = String::new();
        for result in TextDecoder::new(&chars) {
            decoded
                .push(result.map_err(to_js_err)?)
                .map_err(to_js_err)?;
        }
        Ok(JsValue::from_str(&decoded))
    } else {
        // Try as numeric dollcode
        let num = from_dollcode(&chars).map_err(to_js_err)?;

        let mut result: String<OUTPUT_SIZE> = String::new();
        write!(&mut result, "d:{},h:0x{:x}", num, num).map_err(to_js_err)?;

        Ok(JsValue::from_str(&result))
    }
}

// Initialize panic hook
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}
