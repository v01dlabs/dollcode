#![no_std]
#![forbid(unsafe_code)]

use core::fmt::Write;
use dollcode_core::{
    from_dollcode,
    text::{TextDecoder, TextIterator, SEGMENT_SIZE},
    to_dollcode,
};
use heapless::{String, Vec};
use wasm_bindgen::prelude::*;

// Fixed capacity constants
const OUTPUT_SIZE: usize = 128;
const CHAR_BUF_SIZE: usize = 512;

// Simple error conversion
fn to_js_err(_: impl core::fmt::Debug) -> JsValue {
    JsValue::from_str("conversion error")
}

#[wasm_bindgen]
pub fn convert_decimal(input: &str) -> Result<JsValue, JsValue> {
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
    let num = u64::from_str_radix(input.trim_start_matches("0x"), 16).map_err(to_js_err)?;

    let dollcode = to_dollcode(num).map_err(to_js_err)?;

    let mut output: String<OUTPUT_SIZE> = String::new();
    for &c in dollcode.as_chars() {
        output.push(c).map_err(to_js_err)?;
    }

    Ok(JsValue::from_str(&output))
}

#[wasm_bindgen]
pub fn convert_text(input: &str) -> Result<JsValue, JsValue> {
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
    let mut chars: Vec<char, CHAR_BUF_SIZE> = Vec::new();
    for c in input.chars() {
        chars.push(c).map_err(to_js_err)?;
    }

    // If it looks like encoded text (multiple of 5 chars)
    if chars.len() % SEGMENT_SIZE == 0 {
        let mut decoded: String<128> = String::new();
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
