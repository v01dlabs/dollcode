# dollcode ▖▘▌

This is a zero-allocation implementation of dollcode; a trinary encoding system using Unicode box-drawing characters (▖, ▘, and ▌)

## Features ✨

* ⚡ Zero-allocation core with heapless implementation
* 🔢 Support for decimal, hexadecimal, and text encoding (ASCII printable characters)
* 🔄 Bidirectional conversion between text/numbers and dollcode
* 🦀 Pure Rust implementation with no unsafe code
* 🔗 WebAssembly bindings
* 📝 Comprehensive documentation and test coverage

### Web 🌐

Visit the web interface and start converting! The interface supports all available modes for encoding and decoding.

## How it Works 🎛️

Each dollcode character represents a trinary digit (base-3):
* Characters map to values: ▖=1, ▘=2, ▌=3
* Text encoding uses zero-width joiners as delimiters

### Memory Guarantees 🤝

**Zero Allocation**:
* Number encoding/decoding uses fixed-size stack buffers
* Text processing operates on slices without allocation
* Internal operations use stack-only arithmetic
* String conversion and collection may allocate - use heapless types for fully stack-based operations

**Zero Copy**:
* Input data is processed in-place without copying
* Text operations work directly on input slices
* Output is generated directly into fixed-size buffers
* No intermediate copying or reallocation occurs

**Fixed Memory Usage**:
* Number encoding: MAX_DOLLCODE_SIZE chars (41 bytes fixed)
* Text segments: 6 chars per segment (fixed)
* Total output buffer: 1800 bytes (100 chars × 18 bytes)
* Each character produces 5 dollcode chars + 1 delimiter

### Input Limits & Validation ✅

**Text**:
* ASCII printable characters only (codes 32-126)
* Maximum length: 100 characters
* Each char produces 5 dollcode chars + 1 delimiter
* Fixed 18-byte UTF-8 output per input char

**Numbers**:
* Decimal: 0 to 18,446,744,073,709,551,615 (u64::MAX)
* Maximum decimal digits: 20
* Hex: 0x0 to 0xFFFFFFFFFFFFFFFF
* Maximum hex length: 18 chars (including 0x prefix)

**dollcode**:
* Maximum length: 41 chars for numbers (2^64 - 1)
* Text mode: up to 1800 bytes total
* Only valid characters: ▖, ▘, ▌
* Zero-width joiners (\u{200D}) are used as a delimiter

**Error Handling**:
* Comprehensive validation for all inputs
* Buffer overflow protection
* Invalid character detection
* Position tracking for error reporting
* Clear error messages with context

## License 📄

This project is licensed under:

[![License: MPL 2.0](https://img.shields.io/badge/License-MPL%202.0-brightgreen.svg)](LICENSE)

## Credits 🙏

[noe](https://noe.sh/dollcode/) for the original idea & implementation

▘▘▖▘‍▌▖▘▖‍▌▖▌▖‍▌▖▌▘‍▌▖▘‍▌▖▘▘‍▖▖▖▖▖‍▌▖▘‍▌▖▌▘‍▌▘▌▖‍
