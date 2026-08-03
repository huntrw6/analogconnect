use std::{
    fs::File,
    io::Read,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
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
    claimed: AtomicBool,
}

impl MediaSessionGrant {
    pub fn issue<R: RandomSource>(
        random: &mut R,
        lifetime: Duration,
    ) -> Result<(Self, MediaSessionEnrollment), MediaSessionAuthError> {
        if lifetime < Duration::from_secs(1) || lifetime > MAX_LIFETIME {
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
                claimed: AtomicBool::new(false),
            },
            enrollment,
        ))
    }

    #[must_use]
    pub fn authorize(&self, presented: &str, now: Instant) -> bool {
        let (candidate, valid_encoding) = decode_hex::<TOKEN_BYTES>(presented);
        let token_matches = self.token.ct_eq(&candidate).unwrap_u8() == 1;
        valid_encoding
            && token_matches
            && now < self.expires_at
            && !self.revoked.load(Ordering::Acquire)
    }

    pub fn revoke(&self) {
        self.revoked.store(true, Ordering::Release);
    }

    fn claim(self: &Arc<Self>, presented: &str, now: Instant) -> Option<MediaSessionLease> {
        if !self.authorize(presented, now)
            || self
                .claimed
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
        {
            return None;
        }
        Some(MediaSessionLease {
            grant: Arc::clone(self),
        })
    }

    fn is_active(&self, now: Instant) -> bool {
        now < self.expires_at && !self.revoked.load(Ordering::Acquire)
    }
}

impl std::fmt::Debug for MediaSessionGrant {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MediaSessionGrant")
            .field("credential", &"[REDACTED]")
            .field("revoked", &self.revoked.load(Ordering::Relaxed))
            .field("claimed", &self.claimed.load(Ordering::Relaxed))
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

/// Exclusive ownership of a media connection. Dropping it releases the grant so
/// a reconnect can claim the same still-valid session.
pub struct MediaSessionLease {
    grant: Arc<MediaSessionGrant>,
}

impl MediaSessionLease {
    #[must_use]
    pub fn is_active(&self, now: Instant) -> bool {
        self.grant.is_active(now)
    }
}

impl Drop for MediaSessionLease {
    fn drop(&mut self) {
        self.grant.claimed.store(false, Ordering::Release);
    }
}

impl std::fmt::Debug for MediaSessionLease {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MediaSessionLease")
            .field("credential", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

struct ActiveSession {
    session_id: [u8; SESSION_ID_BYTES],
    grant: Arc<MediaSessionGrant>,
}

/// Holds at most one active call-media grant. Issuing a replacement revokes the
/// prior grant before it becomes unreachable.
pub struct MediaSessionRegistry {
    active: Mutex<Option<ActiveSession>>,
}

impl MediaSessionRegistry {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            active: Mutex::new(None),
        }
    }

    pub fn issue<R: RandomSource>(
        &self,
        random: &mut R,
        lifetime: Duration,
    ) -> Result<MediaSessionEnrollment, MediaSessionRegistryError> {
        let (grant, enrollment) = MediaSessionGrant::issue(random, lifetime)?;
        let (session_id, valid_id) = decode_hex::<SESSION_ID_BYTES>(&enrollment.session_id);
        debug_assert!(valid_id);
        let replacement = ActiveSession {
            session_id,
            grant: Arc::new(grant),
        };
        let mut active = self
            .active
            .lock()
            .map_err(|_| MediaSessionRegistryError::Unavailable)?;
        if let Some(previous) = active.replace(replacement) {
            previous.grant.revoke();
        }
        Ok(enrollment)
    }

    pub fn claim(
        &self,
        session_id: &str,
        token: &str,
        now: Instant,
    ) -> Result<MediaSessionLease, MediaSessionRegistryError> {
        let grant = {
            let active = self
                .active
                .lock()
                .map_err(|_| MediaSessionRegistryError::Unavailable)?;
            let Some(active) = active.as_ref() else {
                return Err(MediaSessionRegistryError::Unauthorized);
            };
            let (candidate_id, valid_id) = decode_hex::<SESSION_ID_BYTES>(session_id);
            let id_matches = active.session_id.ct_eq(&candidate_id).unwrap_u8() == 1;
            if !valid_id || !id_matches {
                return Err(MediaSessionRegistryError::Unauthorized);
            }
            Arc::clone(&active.grant)
        };
        grant
            .claim(token, now)
            .ok_or(MediaSessionRegistryError::Unauthorized)
    }

    pub fn revoke(&self, session_id: &str) -> Result<bool, MediaSessionRegistryError> {
        let mut active = self
            .active
            .lock()
            .map_err(|_| MediaSessionRegistryError::Unavailable)?;
        let (candidate_id, valid_id) = decode_hex::<SESSION_ID_BYTES>(session_id);
        let id_matches = active
            .as_ref()
            .is_some_and(|active| active.session_id.ct_eq(&candidate_id).unwrap_u8() == 1);
        if !valid_id || !id_matches {
            return Ok(false);
        }
        let removed = active.take().unwrap();
        removed.grant.revoke();
        Ok(true)
    }
}

impl Default for MediaSessionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for MediaSessionRegistry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MediaSessionRegistry")
            .field("session_details", &"[REDACTED]")
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

fn decode_hex<const N: usize>(encoded: &str) -> ([u8; N], bool) {
    let mut decoded = [0_u8; N];
    if encoded.len() != N * 2 || !encoded.is_ascii() {
        return (decoded, false);
    }
    for (index, pair) in encoded.as_bytes().chunks_exact(2).enumerate() {
        let Some(high) = decode_nibble(pair[0]) else {
            return ([0_u8; N], false);
        };
        let Some(low) = decode_nibble(pair[1]) else {
            return ([0_u8; N], false);
        };
        decoded[index] = (high << 4) | low;
    }
    (decoded, true)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum MediaSessionRegistryError {
    #[error(transparent)]
    Authorization(#[from] MediaSessionAuthError),
    #[error("media session is unauthorized or already in use")]
    Unauthorized,
    #[error("media-session registry is unavailable")]
    Unavailable,
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
        for lifetime in [
            Duration::ZERO,
            Duration::from_nanos(1),
            MAX_LIFETIME + Duration::from_secs(1),
        ] {
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

    #[test]
    fn registry_enforces_one_session_and_one_connection() {
        let registry = MediaSessionRegistry::new();
        let mut random = FixtureRandom {
            next: 1,
            fails: false,
        };
        let enrollment = registry
            .issue(&mut random, Duration::from_secs(30))
            .unwrap();
        let lease = registry
            .claim(enrollment.session_id(), enrollment.token(), Instant::now())
            .unwrap();
        assert!(lease.is_active(Instant::now()));
        assert!(matches!(
            registry.claim(enrollment.session_id(), enrollment.token(), Instant::now()),
            Err(MediaSessionRegistryError::Unauthorized)
        ));
        drop(lease);
        assert!(
            registry
                .claim(enrollment.session_id(), enrollment.token(), Instant::now())
                .is_ok()
        );
    }

    #[test]
    fn replacement_and_teardown_revoke_existing_leases() {
        let registry = MediaSessionRegistry::new();
        let mut random = FixtureRandom {
            next: 2,
            fails: false,
        };
        let first = registry
            .issue(&mut random, Duration::from_secs(30))
            .unwrap();
        let lease = registry
            .claim(first.session_id(), first.token(), Instant::now())
            .unwrap();
        let second = registry
            .issue(&mut random, Duration::from_secs(30))
            .unwrap();
        assert!(!lease.is_active(Instant::now()));
        assert!(matches!(
            registry.claim(first.session_id(), first.token(), Instant::now()),
            Err(MediaSessionRegistryError::Unauthorized)
        ));
        let second_lease = registry
            .claim(second.session_id(), second.token(), Instant::now())
            .unwrap();
        assert!(registry.revoke(second.session_id()).unwrap());
        assert!(!second_lease.is_active(Instant::now()));
        assert!(!registry.revoke(second.session_id()).unwrap());
    }

    #[test]
    fn registry_diagnostics_disclose_no_session_material() {
        let registry = MediaSessionRegistry::new();
        let mut random = FixtureRandom {
            next: 3,
            fails: false,
        };
        let enrollment = registry
            .issue(&mut random, Duration::from_secs(30))
            .unwrap();
        let lease = registry
            .claim(enrollment.session_id(), enrollment.token(), Instant::now())
            .unwrap();
        let debug = format!("{registry:?} {lease:?}");
        assert!(!debug.contains(enrollment.session_id()));
        assert!(!debug.contains(enrollment.token()));
    }
}
