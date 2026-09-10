//! Hosted free-scan scanner (ADR-012) — result tokens.
//!
//! Results are addressable only by an unguessable token with ≥ 128 bits of
//! entropy (ADR-012 §4). Tokens are not derived from the submitted URL or
//! any counter: enumeration and guessing are the only attack paths, and both
//! are priced out by entropy.

use thiserror::Error;

/// Errors from token handling.
#[derive(Debug, Error)]
#[error("invalid result token format")]
pub struct TokenError;

/// Token length in bytes (128-bit entropy = 16 bytes → 22 base64url chars
/// without padding).
const TOKEN_BYTES: usize = 16;

/// Generate a fresh, unguessable result token (base64url, no padding).
///
/// # Examples
///
/// ```
/// let t = crawlkit_scanner::new_result_token();
/// assert!(t.len() >= 22);
/// assert!(!t.contains('=') && !t.contains('+') && !t.contains('/'));
/// ```
pub fn new_result_token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; TOKEN_BYTES];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Validate a token's shape before using it as a storage key (constant-ish
/// rejection: any malformed input is rejected before lookup).
///
/// # Errors
///
/// Returns [`TokenError`] when the token is not well-formed base64url of the
/// expected length.
pub fn parse_result_token(token: &str) -> Result<[u8; TOKEN_BYTES], TokenError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(token)
        .map_err(|_| TokenError)?;
    if bytes.len() != TOKEN_BYTES {
        return Err(TokenError);
    }
    let mut out = [0u8; TOKEN_BYTES];
    out.copy_from_slice(&bytes);
    Ok(out)
}

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_unique_and_wellformed() {
        let a = new_result_token();
        let b = new_result_token();
        assert_ne!(a, b);
        assert!(parse_result_token(&a).is_ok());
        assert!(parse_result_token(&b).is_ok());
    }

    #[test]
    fn tokens_have_no_ambiguous_chars() {
        for _ in 0..32 {
            let t = new_result_token();
            assert!(!t.contains('='));
            assert!(!t.contains('+'));
            assert!(!t.contains('/'));
        }
    }

    #[test]
    fn parse_rejects_malformed_tokens() {
        assert!(parse_result_token("").is_err());
        assert!(parse_result_token("short").is_err());
        assert!(parse_result_token(&"x".repeat(64)).is_err());
        assert!(parse_result_token("../../etc/passwd").is_err());
        // Trailing '=' padding is not part of our canonical form.
        let padded = format!("{}=", new_result_token());
        assert!(parse_result_token(&padded).is_err());
    }
}
