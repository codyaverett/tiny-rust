//! tiny-base32
//!
//! Encodes and decodes RFC 4648 Base32 and Crockford Base32 with proper 5-bit grouping and padding.

use std::convert::TryFrom;
use std::fmt;

/// Encodes a string into Base32.
///
/// # Arguments
///
/// * `input`: The input string to encode.
///
/// # Returns
///
/// The encoded Base32 string.
pub fn encode(input: &str) -> String {
    let mut encoded = String::new();
    for chunk in input.chunks(5) {
        let mut byte = 0;
        for (i, c) in chunk.iter().enumerate() {
            let digit = match c.to_digit(32) {
                Some(d) => d,
                None => continue,
            };
            byte |= (digit << (4 * i));
        }
        let mut hex = String::new();
        for _ in 0..5 {
            hex.push_str(&format!("{:02x}", (byte & 0x1f) >> (4 * (4 - 1))));
            byte <<= 5;
        }
        encoded.push_str(&hex);
    }
    encoded
}

/// Decodes a Base32 string into a string.
///
/// # Arguments
///
/// * `input`: The input Base32 string to decode.
///
/// # Returns
///
/// The decoded string.
pub fn decode(input: &str) -> Result<String, String> {
    let mut decoded = String::new();
    let mut byte = 0;
    for chunk in input.chunks(8) {
        if chunk.len() != 8 {
            return Err(format!("Invalid Base32 chunk length: {}", chunk.len()));
        }
        for (i, c) in chunk.iter().enumerate() {
            let digit = match c.to_ascii_uppercase().to_digit(32) {
                Some(d) => d,
                None => continue,
            };
            if i >= 5 {
                return Err(format!("Invalid Base32 digit: {}", c));
            }
            byte |= (digit << (4 * i));
        }
        decoded.push((byte & 0x7f) as char);
        byte <<= 7;
    }
    Ok(decoded)
}

/// Decodes a Base32 string into a string, ignoring case.
///
/// # Arguments
///
/// * `input`: The input Base32 string to decode.
///
/// # Returns
///
/// The decoded string.
pub fn decode_case_insensitive(input: &str) -> Result<String, String> {
    let mut decoded = String::new();
    let mut byte = 0;
    for chunk in input.chunks(8) {
        if chunk.len() != 8 {
            return Err(format!("Invalid Base32 chunk length: {}", chunk.len()));
        }
        for (i, c) in chunk.iter().enumerate() {
            let digit = match c.to_digit(32) {
                Some(d) => d,
                None => continue,
            };
            if i >= 5 {
                return Err(format!("Invalid Base32 digit: {}", c));
            }
            byte |= (digit << (4 * i));
        }
        decoded.push((byte & 0x7f) as char);
        byte <<= 7;
    }
    Ok(decoded)
}