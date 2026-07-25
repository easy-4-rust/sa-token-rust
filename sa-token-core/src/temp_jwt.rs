//! Stateless temporary tokens backed by HS256 JWT.

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{SaTokenError, SaTokenResult};

pub const TEMP_JWT_VALUE_CLAIM: &str = "value_";
pub const TEMP_JWT_EXPIRY_CLAIM: &str = "eff";
pub const NEVER_EXPIRE: i64 = -1;
pub const NOT_VALUE_EXPIRE: i64 = -2;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TempJwtClaims {
    #[serde(rename = "value_")]
    value: Value,
    eff: i64,
}

/// Java-compatible stateless temporary JWT template.
///
/// The signing key uses the Java module's `MD5(secret)` hexadecimal key
/// derivation so tokens can be shared during migration.
#[derive(Clone)]
pub struct TempJwtTemplate {
    signing_key: Vec<u8>,
}

impl TempJwtTemplate {
    pub fn new(secret: impl AsRef<str>) -> SaTokenResult<Self> {
        let secret = secret.as_ref();
        if secret.trim().is_empty() {
            return Err(SaTokenError::ConfigError(
                "non-empty JWT secret is required for temporary JWT".to_string(),
            ));
        }
        let mut hasher = Md5::new();
        hasher.update(secret.as_bytes());
        let signing_key = hex::encode(hasher.finalize()).into_bytes();
        Ok(Self { signing_key })
    }

    pub fn create_token(&self, value: Value, timeout_seconds: i64) -> SaTokenResult<String> {
        self.create_token_with_index(value, timeout_seconds, false)
    }

    /// `record_index` is accepted for Java API compatibility. Stateless JWT
    /// cannot maintain a reverse index, so the value has no effect.
    pub fn create_token_with_index(
        &self,
        value: Value,
        timeout_seconds: i64,
        _record_index: bool,
    ) -> SaTokenResult<String> {
        if timeout_seconds < NEVER_EXPIRE {
            return Err(SaTokenError::ConfigError(
                "temporary JWT timeout must be -1 or non-negative".to_string(),
            ));
        }
        let eff = if timeout_seconds == NEVER_EXPIRE {
            NEVER_EXPIRE
        } else {
            now_millis().saturating_add(timeout_seconds.saturating_mul(1000))
        };
        let claims = TempJwtClaims { value, eff };
        encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(&self.signing_key),
        )
        .map_err(|error| SaTokenError::TokenGenerationFailed(error.to_string()))
    }

    pub fn parse_token(&self, token: &str) -> SaTokenResult<Value> {
        let claims = self.decode_claims(token)?;
        if claims.eff != NEVER_EXPIRE && claims.eff < now_millis() {
            return Err(SaTokenError::TokenExpired);
        }
        Ok(claims.value)
    }

    pub fn get_timeout(&self, token: &str) -> SaTokenResult<i64> {
        let claims = self.decode_claims(token)?;
        if claims.eff == NEVER_EXPIRE {
            return Ok(NEVER_EXPIRE);
        }
        let remaining = claims.eff.saturating_sub(now_millis());
        if remaining < 0 {
            return Ok(NOT_VALUE_EXPIRE);
        }
        Ok(remaining / 1000)
    }

    pub fn delete_token(&self, _token: &str) -> SaTokenResult<()> {
        Err(SaTokenError::ApiDisabled)
    }

    pub fn get_token_list(&self, _value: &Value) -> SaTokenResult<Vec<String>> {
        Err(SaTokenError::ApiDisabled)
    }

    fn decode_claims(&self, token: &str) -> SaTokenResult<TempJwtClaims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.required_spec_claims.clear();
        validation.validate_exp = false;
        decode::<TempJwtClaims>(
            token,
            &DecodingKey::from_secret(&self.signing_key),
            &validation,
        )
        .map(|data| data.claims)
        .map_err(|error| SaTokenError::InvalidToken(error.to_string()))
    }
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_typed_json_value_and_timeout() {
        let template = TempJwtTemplate::new("migration-secret").unwrap();
        let value = serde_json::json!({"user_id": 1001, "purpose": "email"});
        let token = template.create_token(value.clone(), 60).unwrap();

        assert_eq!(template.parse_token(&token).unwrap(), value);
        assert!((0..=60).contains(&template.get_timeout(&token).unwrap()));
    }

    #[test]
    fn permanent_and_expired_semantics_match_java() {
        let template = TempJwtTemplate::new("migration-secret").unwrap();
        let permanent = template
            .create_token(Value::String("permanent".to_string()), NEVER_EXPIRE)
            .unwrap();
        assert_eq!(template.get_timeout(&permanent).unwrap(), NEVER_EXPIRE);

        let expired = template
            .create_token(Value::String("expired".to_string()), 0)
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        assert!(matches!(
            template.parse_token(&expired),
            Err(SaTokenError::TokenExpired)
        ));
        assert_eq!(template.get_timeout(&expired).unwrap(), NOT_VALUE_EXPIRE);
    }

    #[test]
    fn stateless_operations_are_explicitly_disabled() {
        let template = TempJwtTemplate::new("migration-secret").unwrap();
        assert!(matches!(
            template.delete_token("token"),
            Err(SaTokenError::ApiDisabled)
        ));
        assert!(matches!(
            template.get_token_list(&Value::Null),
            Err(SaTokenError::ApiDisabled)
        ));
    }
}
