//! tiny-base32 tests
//!
//! Tests the encoding and decoding functions of the `tiny-base32` crate.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode() {
        let input = "Hello, World!";
        let encoded = encode(input);
        assert_eq!(encoded, "JBSWY3DPEB3W64TMMQ==");
    }

    #[test]
    fn test_decode() {
        let input = "JBSWY3DPEB3W64TMMQ==";
        let decoded = decode(input).unwrap();
        assert_eq!(decoded, "Hello, World!");
    }

    #[test]
    fn test_decode_case_insensitive() {
        let input = "jBSWY3DPEB3W64tMMq==";
        let decoded = decode_case_insensitive(input).unwrap();
        assert_eq!(decoded, "Hello, World!");
    }

    #[test]
    fn test_invalid_input() {
        let input = "Invalid Input!";
        let result = decode(input);
        assert!(result.is_err());
    }
}