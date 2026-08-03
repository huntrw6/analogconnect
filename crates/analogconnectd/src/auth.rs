use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
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

#[derive(Clone)]
pub struct AuthTokens {
    current: AuthToken,
    previous: Option<AuthToken>,
}

impl AuthTokens {
    #[must_use]
    pub const fn new(current: AuthToken) -> Self {
        Self {
            current,
            previous: None,
        }
    }

    #[must_use]
    pub const fn with_previous(current: AuthToken, previous: AuthToken) -> Self {
        Self {
            current,
            previous: Some(previous),
        }
    }

    #[must_use]
    pub fn matches(&self, candidate: &str) -> bool {
        let current = self.current.matches(candidate);
        let previous = self
            .previous
            .as_ref()
            .is_some_and(|token| token.matches(candidate));
        current | previous
    }
}

impl std::fmt::Debug for AuthTokens {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("AuthTokens([redacted])")
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

struct MutationWindow {
    started_at: Instant,
    accepted: u32,
}

pub struct MutationLimiter {
    maximum: u32,
    window: Duration,
    state: Mutex<MutationWindow>,
}

impl MutationLimiter {
    #[must_use]
    pub fn new(maximum: u32, window: Duration) -> Self {
        Self {
            maximum,
            window,
            state: Mutex::new(MutationWindow {
                started_at: Instant::now(),
                accepted: 0,
            }),
        }
    }

    pub fn allow(&self) -> Result<bool, MutationLimiterError> {
        self.allow_at(Instant::now())
    }

    fn allow_at(&self, now: Instant) -> Result<bool, MutationLimiterError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| MutationLimiterError::LockPoisoned)?;
        if now.saturating_duration_since(state.started_at) >= self.window {
            state.started_at = now;
            state.accepted = 0;
        }
        if state.accepted >= self.maximum {
            return Ok(false);
        }
        state.accepted = state.accepted.saturating_add(1);
        Ok(true)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum MutationLimiterError {
    #[error("mutation rate limiter is unavailable")]
    LockPoisoned,
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

    #[test]
    fn mutation_limiter_bounds_and_resets_without_payload_state() {
        let limiter = MutationLimiter::new(2, Duration::from_secs(60));
        let start = Instant::now();
        assert!(limiter.allow_at(start).unwrap());
        assert!(limiter.allow_at(start + Duration::from_secs(1)).unwrap());
        assert!(!limiter.allow_at(start + Duration::from_secs(2)).unwrap());
        assert!(limiter.allow_at(start + Duration::from_secs(61)).unwrap());
    }

    #[test]
    fn token_set_supports_staged_rotation_without_debug_disclosure() {
        let current_text = "current-current-current-current-0001";
        let previous_text = "previous-previous-previous-prev-0001";
        let tokens = AuthTokens::with_previous(
            AuthToken::new(current_text).unwrap(),
            AuthToken::new(previous_text).unwrap(),
        );
        assert!(tokens.matches(current_text));
        assert!(tokens.matches(previous_text));
        assert!(!tokens.matches("wrong"));
        let debug = format!("{tokens:?}");
        assert!(!debug.contains(current_text));
        assert!(!debug.contains(previous_text));
    }
}
