use subtle::ConstantTimeEq;
use thiserror::Error;

#[derive(Clone)]
pub struct AuthToken(Vec<u8>);

impl AuthToken {
    pub fn new(token: impl AsRef<str>) -> Result<Self, AuthTokenError> {
        let bytes = token.as_ref().as_bytes();
        if bytes.len() < 32 || bytes.len() > 256 {
            return Err(AuthTokenError::InvalidLength);
        }
        Ok(Self(bytes.to_vec()))
    }

    #[must_use]
    pub fn matches(&self, candidate: &str) -> bool {
        let candidate = candidate.as_bytes();
        if candidate.len() != self.0.len() {
            return false;
        }
        bool::from(self.0.ct_eq(candidate))
    }
}

impl std::fmt::Debug for AuthToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("AuthToken([redacted])")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum AuthTokenError {
    #[error("API token must contain between 32 and 256 bytes")]
    InvalidLength,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token_text() -> String {
        [
            "synthetic",
            "-test",
            "-token",
            "-not",
            "-a",
            "-credential",
            "-0001",
        ]
        .concat()
    }

    #[test]
    fn validates_and_matches_without_debug_disclosure() {
        let text = token_text();
        let token = AuthToken::new(&text).unwrap();
        assert!(token.matches(&text));
        assert!(!token.matches("wrong"));
        assert_eq!(format!("{token:?}"), "AuthToken([redacted])");
        assert!(!format!("{token:?}").contains(&text));
    }

    #[test]
    fn rejects_short_tokens() {
        assert_eq!(
            AuthToken::new("too-short").unwrap_err(),
            AuthTokenError::InvalidLength
        );
    }
}
