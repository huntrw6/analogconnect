package com.analogconnect.client;

final class MediaSessionCredentials {
    static final long MAX_LIFETIME_SECONDS = 5 * 60;

    private final String sessionId;
    private final String token;
    private final int wireFormat;
    private final long expiresAtMonotonicMillis;

    MediaSessionCredentials(String sessionId, String token, long lifetimeSeconds,
            long issuedAtMonotonicMillis) throws CredentialException {
        this(sessionId, token, lifetimeSeconds, issuedAtMonotonicMillis, "hfp_wideband");
    }

    MediaSessionCredentials(String sessionId, String token, long lifetimeSeconds,
            long issuedAtMonotonicMillis, String audioFormat) throws CredentialException {
        if (!isHex(sessionId, 32) || !isHex(token, 64)) {
            throw new CredentialException("media session credential is invalid");
        }
        if (lifetimeSeconds < 1 || lifetimeSeconds > MAX_LIFETIME_SECONDS
                || issuedAtMonotonicMillis < 0) {
            throw new CredentialException("media session lifetime is invalid");
        }
        long lifetimeMillis;
        try {
            lifetimeMillis = Math.multiplyExact(lifetimeSeconds, 1_000L);
            this.expiresAtMonotonicMillis = Math.addExact(
                    issuedAtMonotonicMillis, lifetimeMillis);
        } catch (ArithmeticException overflow) {
            throw new CredentialException("media session lifetime is invalid");
        }
        this.sessionId = sessionId;
        this.token = token;
        if ("hfp_narrowband".equals(audioFormat)) {
            wireFormat = AudioPacketCodec.FORMAT_NARROWBAND;
        } else if ("hfp_wideband".equals(audioFormat)) {
            wireFormat = AudioPacketCodec.FORMAT_WIDEBAND;
        } else {
            throw new CredentialException("media session audio format is invalid");
        }
    }

    String sessionId() {
        return sessionId;
    }

    String token() {
        return token;
    }

    int wireFormat() {
        return wireFormat;
    }

    boolean isExpired(long monotonicMillis) {
        return monotonicMillis >= expiresAtMonotonicMillis;
    }

    @Override
    public String toString() {
        return "MediaSessionCredentials{sessionId=[REDACTED], token=[REDACTED]}";
    }

    private static boolean isHex(String value, int length) {
        if (value == null || value.length() != length) {
            return false;
        }
        for (int index = 0; index < value.length(); index++) {
            char character = value.charAt(index);
            boolean digit = character >= '0' && character <= '9';
            boolean lower = character >= 'a' && character <= 'f';
            boolean upper = character >= 'A' && character <= 'F';
            if (!digit && !lower && !upper) {
                return false;
            }
        }
        return true;
    }

    static final class CredentialException extends Exception {
        CredentialException(String message) {
            super(message);
        }
    }
}
