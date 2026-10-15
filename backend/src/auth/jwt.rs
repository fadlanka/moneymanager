use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const TOKEN_DAYS: i64 = 30;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: Uuid,
    exp: i64,
    iat: i64,
}

pub struct JwtKeys {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl JwtKeys {
    pub fn new(secret: &str) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret.as_bytes()),
            decoding: DecodingKey::from_secret(secret.as_bytes()),
        }
    }

    pub fn issue(&self, user_id: Uuid) -> jsonwebtoken::errors::Result<String> {
        let now = Utc::now();
        let claims = Claims {
            sub: user_id,
            iat: now.timestamp(),
            exp: (now + Duration::days(TOKEN_DAYS)).timestamp(),
        };
        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)
    }

    pub fn verify(&self, token: &str) -> jsonwebtoken::errors::Result<Uuid> {
        let data = decode::<Claims>(token, &self.decoding, &Validation::new(Algorithm::HS256))?;
        Ok(data.claims.sub)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_wrong_key() {
        let keys = JwtKeys::new("a-very-long-test-secret-of-32-chars!!");
        let id = Uuid::new_v4();
        let token = keys.issue(id).unwrap();
        assert_eq!(keys.verify(&token).unwrap(), id);
        let other = JwtKeys::new("another-very-long-secret-of-32-chars!");
        assert!(other.verify(&token).is_err());
    }
}
