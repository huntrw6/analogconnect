use std::{
    fs::File,
    io::Read,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use subtle::ConstantTimeEq;
use thiserror::Error;

const SESSION_ID_BYTES: usize = 16;
const TOKEN_BYTES: usize = 32;
const ENTROPY_BYTES: usize = SESSION_ID_BYTES + TOKEN_BYTES;
const MAX_LIFETIME: Duration = Duration::from_secs(5 * 60);

pub trait RandomSource {
    type Error;

    fn fill(&mut self, bytes: &mut [u8]) -> Result<(), Self::Error>;
}

pub struct OsRandomSource;

impl RandomSource for OsRandomSource {
    type Error = std::io::Error;

    fn fill(&mut self, bytes: &mut [u8]) -> Result<(), Self::Error> {
        File::open("/dev/urandom")?.read_exact(bytes)
    }
}

/// Server-side authorization state for one audio session. Debug output contains
/// neither the credential nor the session identifier.
pub struct MediaSessionGrant {
    token: [u8; TOKEN_BYTES],
    expires_at: Instant,
    revoked: AtomicBool,
}

impl MediaSessionGrant {
    pub fn issue<R: RandomSource>(
        random: &mut R,
        lifetime: Duration,
    ) -> Result<(Self, MediaSessionEnrollment), MediaSessionAuthError> {
        if lifetime.is_zero() || lifetime > MAX_LIFETIME {
            return Err(MediaSessionAuthError::InvalidLifetime);
        }
        let mut entropy = [0_u8; ENTROPY_BYTES];
        random
            .fill(&mut entropy)
            .map_err(|_| MediaSessionAuthError::RandomUnavailable)?;
        let mut token = [0_u8; TOKEN_BYTES];
        token.copy_from_slice(&entropy[SESSION_ID_BYTES..]);
        let enrollment = MediaSessionEnrollment {
            session_id: encode_hex(&entropy[..SESSION_ID_BYTES]),
            token: encode_hex(&token),
            lifetime_seconds: lifetime.as_secs(),
        };
        Ok((
            Self {
                token,
                expires_at: Instant::now() + lifetime,
                revoked: AtomicBool::new(false),
            },
            enrollment,
        ))
    }

    #[must_use]
    pub fn authorize(&self, presented: &str, now: Instant) -> bool {
        let (candidate, valid_encoding) = decode_token(presented);
        let token_matches = self.token.ct_eq(&candidate).unwrap_u8() == 1;
        valid_encoding
            && token_matches
            && now < self.expires_at
            && !self.revoked.load(Ordering::Acquire)
    }

    pub fn revoke(&self) {
        self.revoked.store(true, Ordering::Release);
    }
}

impl std::fmt::Debug for MediaSessionGrant {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MediaSessionGrant")
            .field("credential", &"[REDACTED]")
            .field("revoked", &self.revoked.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

/// One-time response material delivered to an already authenticated client.
/// Callers must never log or persist this value.
pub struct MediaSessionEnrollment {
    session_id: String,
    token: String,
    lifetime_seconds: u64,
}

impl MediaSessionEnrollment {
    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }

    #[must_use]
    pub const fn lifetime_seconds(&self) -> u64 {
        self.lifetime_seconds
    }
}

impl std::fmt::Debug for MediaSessionEnrollment {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MediaSessionEnrollment")
            .field("session_id", &"[REDACTED]")
            .field("credential", &"[REDACTED]")
            .field("lifetime_seconds", &self.lifetime_seconds)
            .finish()
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

fn decode_token(encoded: &str) -> ([u8; TOKEN_BYTES], bool) {
    let mut token = [0_u8; TOKEN_BYTES];
    if encoded.len() != TOKEN_BYTES * 2 || !encoded.is_ascii() {
        return (token, false);
    }
    for (index, pair) in encoded.as_bytes().chunks_exact(2).enumerate() {
        let Some(high) = decode_nibble(pair[0]) else {
            return ([0_u8; TOKEN_BYTES], false);
        };
        let Some(low) = decode_nibble(pair[1]) else {
            return ([0_u8; TOKEN_BYTES], false);
        };
        token[index] = (high << 4) | low;
    }
    (token, true)
}

const fn decode_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum MediaSessionAuthError {
    #[error("media-session lifetime must be between one second and five minutes")]
    InvalidLifetime,
    #[error("operating-system randomness is unavailable")]
    RandomUnavailable,
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixtureRandom {
        next: u8,
        fails: bool,
    }

    impl RandomSource for FixtureRandom {
        type Error = ();

        fn fill(&mut self, bytes: &mut [u8]) -> Result<(), Self::Error> {
            if self.fails {
                return Err(());
            }
            for byte in bytes {
                *byte = self.next;
                self.next = self.next.wrapping_add(1);
            }
            Ok(())
        }
    }

    #[test]
    fn issued_grant_authorizes_only_its_strictly_encoded_token() {
        let mut random = FixtureRandom {
            next: 0,
            fails: false,
        };
        let before_issue = Instant::now();
        let (grant, enrollment) =
            MediaSessionGrant::issue(&mut random, Duration::from_secs(30)).unwrap();
        assert_eq!(enrollment.session_id().len(), SESSION_ID_BYTES * 2);
        assert_eq!(enrollment.token().len(), TOKEN_BYTES * 2);
        assert_eq!(enrollment.lifetime_seconds(), 30);
        assert!(grant.authorize(enrollment.token(), before_issue));
        assert!(!grant.authorize("short", before_issue));
        assert!(!grant.authorize(&"z".repeat(TOKEN_BYTES * 2), before_issue));

        let mut wrong = enrollment.token().to_owned();
        wrong.replace_range(..2, "ff");
        assert!(!grant.authorize(&wrong, before_issue));
    }

    #[test]
    fn grant_expires_and_can_be_revoked_immediately() {
        let mut random = FixtureRandom {
            next: 9,
            fails: false,
        };
        let (grant, enrollment) =
            MediaSessionGrant::issue(&mut random, Duration::from_secs(1)).unwrap();
        assert!(!grant.authorize(enrollment.token(), Instant::now() + Duration::from_secs(2)));
        assert!(grant.authorize(enrollment.token(), Instant::now()));
        grant.revoke();
        assert!(!grant.authorize(enrollment.token(), Instant::now()));
    }

    #[test]
    fn issuance_bounds_lifetime_and_redacts_all_secret_material() {
        let mut random = FixtureRandom {
            next: 31,
            fails: false,
        };
        for lifetime in [Duration::ZERO, MAX_LIFETIME + Duration::from_secs(1)] {
            assert!(matches!(
                MediaSessionGrant::issue(&mut random, lifetime),
                Err(MediaSessionAuthError::InvalidLifetime)
            ));
        }
        let (grant, enrollment) = MediaSessionGrant::issue(&mut random, MAX_LIFETIME).unwrap();
        let debug = format!("{grant:?} {enrollment:?}");
        assert!(!debug.contains(enrollment.session_id()));
        assert!(!debug.contains(enrollment.token()));

        random.fails = true;
        assert!(matches!(
            MediaSessionGrant::issue(&mut random, Duration::from_secs(1)),
            Err(MediaSessionAuthError::RandomUnavailable)
        ));
    }

    #[test]
    fn operating_system_issues_distinct_unlogged_credentials() {
        let (first, first_enrollment) =
            MediaSessionGrant::issue(&mut OsRandomSource, Duration::from_secs(1)).unwrap();
        let (_, second_enrollment) =
            MediaSessionGrant::issue(&mut OsRandomSource, Duration::from_secs(1)).unwrap();
        assert_ne!(
            first_enrollment.session_id(),
            second_enrollment.session_id()
        );
        assert_ne!(first_enrollment.token(), second_enrollment.token());
        assert!(first.authorize(first_enrollment.token(), Instant::now()));
    }
}
