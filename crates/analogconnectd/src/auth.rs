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
}
