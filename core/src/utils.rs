//! Small utility functions shared across the crate.

/// Decode a uint128 from the last 16 bytes of a byte slice.
/// If the slice is shorter than 16 bytes, leading bytes are treated as zero.
pub fn u128_from_be_bytes(bytes: &[u8]) -> u128 {
    let start = bytes.len().saturating_sub(16);
    let mut buf = [0u8; 16];
    buf.copy_from_slice(&bytes[start..start + 16]);
    u128::from_be_bytes(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u128_from_be_bytes_basic() {
        let mut buf = [0u8; 32];
        buf[16..32].copy_from_slice(&1000u128.to_be_bytes());
        assert_eq!(u128_from_be_bytes(&buf), 1000);
    }

    #[test]
    fn test_u128_from_be_bytes_zero() {
        let buf = [0u8; 32];
        assert_eq!(u128_from_be_bytes(&buf), 0);
    }

    #[test]
    fn test_u128_from_be_bytes_short_slice() {
        let buf = 42u128.to_be_bytes();
        assert_eq!(u128_from_be_bytes(&buf), 42);
    }
}
