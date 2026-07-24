// Author: 金书记
//
//! Constant-time comparison utilities | 恒定时间比较工具
//!
//! Prevents timing attacks on secret/token/signature comparisons.
//! Uses the audited `subtle` primitive instead of maintaining custom
//! cryptographic comparison code.

use subtle::ConstantTimeEq;

/// Constant-time equality check for two byte slices.
///
/// Returns `true` if `a == b`, but takes the same amount of time
/// regardless of where the first difference occurs.
///
/// # Important
///
/// Secret lengths should be fixed by their protocol. A length mismatch is
/// rejected by `subtle` before comparing contents.
pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    bool::from(a.ct_eq(b))
}

/// Constant-time equality check for two strings.
pub fn ct_eq_str(a: &str, b: &str) -> bool {
    ct_eq(a.as_bytes(), b.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ct_eq_equal() {
        assert!(ct_eq(b"hello", b"hello"));
        assert!(ct_eq_str("secret", "secret"));
    }

    #[test]
    fn test_ct_eq_not_equal() {
        assert!(!ct_eq(b"hello", b"world"));
        assert!(!ct_eq_str("secret", "secre"));
    }

    #[test]
    fn test_ct_eq_different_length() {
        assert!(!ct_eq(b"short", b"longer string"));
        assert!(!ct_eq_str("a", "ab"));
    }

    #[test]
    fn test_ct_eq_empty() {
        assert!(ct_eq(b"", b""));
        assert!(!ct_eq(b"", b"a"));
    }
}
