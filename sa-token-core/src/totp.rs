//! RFC 6238 time-based one-time passwords.

use hmac::{Hmac, Mac};
use sha1::Sha1;
use sha2_010::{Sha256, Sha512};
use thiserror::Error;

use crate::base32::{self, Base32Error};
use crate::constant_time::ct_eq_str;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TotpAlgorithm {
    Sha1,
    Sha256,
    Sha512,
}

impl TotpAlgorithm {
    fn provisioning_name(self) -> &'static str {
        match self {
            Self::Sha1 => "SHA1",
            Self::Sha256 => "SHA256",
            Self::Sha512 => "SHA512",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TotpError {
    #[error(transparent)]
    InvalidBase32(#[from] Base32Error),
    #[error("TOTP time step must be greater than zero")]
    InvalidTimeStep,
    #[error("TOTP digits must be between 1 and 10")]
    InvalidDigits,
    #[error("TOTP secret length must be greater than zero")]
    InvalidSecretLength,
    #[error("TOTP random generation failed: {0}")]
    RandomGeneration(String),
    #[error("TOTP HMAC key is invalid")]
    InvalidKey,
}

#[derive(Debug, Clone)]
pub struct Totp {
    time_step: u64,
    digits: u32,
    algorithm: TotpAlgorithm,
    secret_length: usize,
}

impl Default for Totp {
    fn default() -> Self {
        Self {
            time_step: 30,
            digits: 6,
            algorithm: TotpAlgorithm::Sha1,
            secret_length: 16,
        }
    }
}

impl Totp {
    pub fn new(
        time_step: u64,
        digits: u32,
        algorithm: TotpAlgorithm,
        secret_length: usize,
    ) -> Result<Self, TotpError> {
        if time_step == 0 {
            return Err(TotpError::InvalidTimeStep);
        }
        if !(1..=10).contains(&digits) {
            return Err(TotpError::InvalidDigits);
        }
        if secret_length == 0 {
            return Err(TotpError::InvalidSecretLength);
        }
        Ok(Self {
            time_step,
            digits,
            algorithm,
            secret_length,
        })
    }

    pub fn time_step(&self) -> u64 {
        self.time_step
    }

    pub fn digits(&self) -> u32 {
        self.digits
    }

    pub fn algorithm(&self) -> TotpAlgorithm {
        self.algorithm
    }

    pub fn generate_secret(&self) -> Result<String, TotpError> {
        let mut secret = vec![0_u8; self.secret_length];
        getrandom::fill(&mut secret)
            .map_err(|error| TotpError::RandomGeneration(error.to_string()))?;
        Ok(base32::encode(&secret))
    }

    pub fn generate(&self, secret: &str) -> Result<String, TotpError> {
        self.generate_at(secret, current_unix_seconds())
    }

    pub fn generate_at(&self, secret: &str, unix_seconds: u64) -> Result<String, TotpError> {
        let secret = base32::decode(secret)?;
        let counter = unix_seconds / self.time_step;
        let message = counter.to_be_bytes();
        let digest = match self.algorithm {
            TotpAlgorithm::Sha1 => hmac_sha1(&secret, &message)?,
            TotpAlgorithm::Sha256 => hmac_sha256(&secret, &message)?,
            TotpAlgorithm::Sha512 => hmac_sha512(&secret, &message)?,
        };
        let offset = usize::from(digest[digest.len() - 1] & 0x0f);
        let binary = (u32::from(digest[offset] & 0x7f) << 24)
            | (u32::from(digest[offset + 1]) << 16)
            | (u32::from(digest[offset + 2]) << 8)
            | u32::from(digest[offset + 3]);
        let modulus = 10_u64.pow(self.digits);
        Ok(format!(
            "{:0width$}",
            u64::from(binary) % modulus,
            width = self.digits as usize
        ))
    }

    pub fn validate(
        &self,
        secret: &str,
        code: &str,
        window_offset: u32,
    ) -> Result<bool, TotpError> {
        Ok(self
            .matched_counter_at(secret, code, window_offset, current_unix_seconds())?
            .is_some())
    }

    /// Validate and return the matched counter.
    ///
    /// Persisting the returned counter per account lets applications reject a
    /// second use of the same otherwise-valid TOTP code.
    pub fn matched_counter_at(
        &self,
        secret: &str,
        code: &str,
        window_offset: u32,
        unix_seconds: u64,
    ) -> Result<Option<u64>, TotpError> {
        if code.len() != self.digits as usize || !code.bytes().all(|byte| byte.is_ascii_digit()) {
            return Ok(None);
        }

        let current = unix_seconds / self.time_step;
        let offset = u64::from(window_offset);
        let start = current.saturating_sub(offset);
        let end = current.saturating_add(offset);
        let mut matched = None;
        for counter in start..=end {
            let calculated = self.generate_at(secret, counter.saturating_mul(self.time_step))?;
            if ct_eq_str(&calculated, code) {
                matched = Some(counter);
            }
        }
        Ok(matched)
    }

    pub fn provisioning_uri(&self, account: &str, issuer: Option<&str>, secret: &str) -> String {
        let label = match issuer {
            Some(issuer) => format!("{issuer}:{account}"),
            None => account.to_string(),
        };
        let mut uri = format!(
            "otpauth://totp/{}?secret={}&algorithm={}&digits={}&period={}",
            urlencoding::encode(&label),
            urlencoding::encode(secret),
            self.algorithm.provisioning_name(),
            self.digits,
            self.time_step
        );
        if let Some(issuer) = issuer {
            uri.push_str("&issuer=");
            uri.push_str(&urlencoding::encode(issuer));
        }
        uri
    }
}

fn hmac_sha1(secret: &[u8], message: &[u8]) -> Result<Vec<u8>, TotpError> {
    let mut mac = Hmac::<Sha1>::new_from_slice(secret).map_err(|_| TotpError::InvalidKey)?;
    mac.update(message);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn hmac_sha256(secret: &[u8], message: &[u8]) -> Result<Vec<u8>, TotpError> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).map_err(|_| TotpError::InvalidKey)?;
    mac.update(message);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn hmac_sha512(secret: &[u8], message: &[u8]) -> Result<Vec<u8>, TotpError> {
    let mut mac = Hmac::<Sha512>::new_from_slice(secret).map_err(|_| TotpError::InvalidKey)?;
    mac.update(message);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn current_unix_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RFC_SECRET: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

    #[test]
    fn rfc6238_sha1_vectors() {
        let totp = Totp::new(30, 8, TotpAlgorithm::Sha1, 20).unwrap();
        for (time, expected) in [
            (59, "94287082"),
            (1_111_111_109, "07081804"),
            (1_111_111_111, "14050471"),
            (1_234_567_890, "89005924"),
            (2_000_000_000, "69279037"),
            (20_000_000_000, "65353130"),
        ] {
            assert_eq!(totp.generate_at(RFC_SECRET, time).unwrap(), expected);
        }
    }

    #[test]
    fn validation_returns_counter_for_replay_tracking() {
        let totp = Totp::default();
        let now = 1_700_000_000;
        let code = totp.generate_at(RFC_SECRET, now).unwrap();
        assert_eq!(
            totp.matched_counter_at(RFC_SECRET, &code, 1, now).unwrap(),
            Some(now / 30)
        );
        assert_eq!(
            totp.matched_counter_at(RFC_SECRET, "abcdef", 1, now)
                .unwrap(),
            None
        );
    }

    #[test]
    fn secret_and_uri_are_google_authenticator_compatible() {
        let totp = Totp::default();
        let secret = totp.generate_secret().unwrap();
        assert_eq!(base32::decode(&secret).unwrap().len(), 16);
        let uri = totp.provisioning_uri("alice@example.com", Some("Sa Token"), &secret);
        assert!(uri.starts_with("otpauth://totp/Sa%20Token%3Aalice%40example.com?"));
        assert!(uri.contains("issuer=Sa%20Token"));
    }
}
