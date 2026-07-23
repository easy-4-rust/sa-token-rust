// Author: 金书记
//
//! Constant-time comparison utilities | 恒定时间比较工具
//!
//! Prevents timing attacks on secret/token/signature comparisons.
//! 不依赖外部 crate，手动实现恒定时间比较。

/// Constant-time equality check for two byte slices.
///
/// Returns `true` if `a == b`, but takes the same amount of time
/// regardless of where the first difference occurs.
///
/// # Important
///
/// If lengths differ, it still performs a full scan to avoid
/// leaking length information via timing.
pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    // XOR all bytes; if lengths differ, result is non-zero
    let max_len = a.len().max(b.len());
    let mut result: u8 = (a.len() ^ b.len()) as u8;
    for i in 0..max_len {
        let byte_a = a.get(i).copied().unwrap_or(0);
        let byte_b = b.get(i).copied().unwrap_or(0);
        result |= byte_a ^ byte_b;
    }
    result == 0
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
