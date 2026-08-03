package com.analogconnect.client;

public final class MediaSessionCredentialsTest {
    public static void main(String[] args) throws Exception {
        acceptsMatchingServerShapeAndExpiresMonotonically();
        acceptsOnlyExplicitHfpFormats();
        rejectsMalformedOrUnboundedValues();
        redactsDiagnostics();
        System.out.println("ANDROID_MEDIA_AUTH_TESTS=PASS tests=4");
    }

    private static void acceptsOnlyExplicitHfpFormats() throws Exception {
        String id = repeat("01", 16);
        String token = repeat("ab", 32);
        MediaSessionCredentials narrow = new MediaSessionCredentials(
                id, token, 30, 0, "hfp_narrowband");
        assertEquals(AudioPacketCodec.FORMAT_NARROWBAND, narrow.wireFormat());
        try {
            new MediaSessionCredentials(id, token, 30, 0, "unknown");
            throw new AssertionError("expected invalid media format");
        } catch (MediaSessionCredentials.CredentialException expected) {
            assertFalse(expected.getMessage().contains(id));
            assertFalse(expected.getMessage().contains(token));
        }
    }

    private static void acceptsMatchingServerShapeAndExpiresMonotonically() throws Exception {
        String id = repeat("01", 16);
        String token = repeat("aB", 32);
        MediaSessionCredentials credentials = new MediaSessionCredentials(id, token, 30, 10_000);
        assertEquals(id, credentials.sessionId());
        assertEquals(token, credentials.token());
        assertFalse(credentials.isExpired(39_999));
        assertTrue(credentials.isExpired(40_000));
    }

    private static void rejectsMalformedOrUnboundedValues() {
        String id = repeat("01", 16);
        String token = repeat("ab", 32);
        expectFailure("short", token, 1, 0);
        expectFailure(id, "short", 1, 0);
        expectFailure(repeat("zz", 16), token, 1, 0);
        expectFailure(id, token, 0, 0);
        expectFailure(id, token, 301, 0);
        expectFailure(id, token, 1, -1);
    }

    private static void redactsDiagnostics() throws Exception {
        String id = repeat("12", 16);
        String token = repeat("34", 32);
        String diagnostic = new MediaSessionCredentials(id, token, 1, 0).toString();
        assertFalse(diagnostic.contains(id));
        assertFalse(diagnostic.contains(token));
        assertTrue(diagnostic.contains("[REDACTED]"));
    }

    private static void expectFailure(String id, String token, long lifetime, long issuedAt) {
        try {
            new MediaSessionCredentials(id, token, lifetime, issuedAt);
            throw new AssertionError("expected invalid media credential");
        } catch (MediaSessionCredentials.CredentialException expected) {
            assertFalse(expected.getMessage().contains(id));
            assertFalse(expected.getMessage().contains(token));
        }
    }

    private static String repeat(String value, int count) {
        StringBuilder output = new StringBuilder(value.length() * count);
        for (int index = 0; index < count; index++) {
            output.append(value);
        }
        return output.toString();
    }

    private static void assertEquals(String expected, String actual) {
        if (!expected.equals(actual)) {
            throw new AssertionError("values differ");
        }
    }

    private static void assertEquals(int expected, int actual) {
        if (expected != actual) {
            throw new AssertionError("values differ");
        }
    }

    private static void assertTrue(boolean value) {
        if (!value) {
            throw new AssertionError("expected true");
        }
    }

    private static void assertFalse(boolean value) {
        if (value) {
            throw new AssertionError("expected false");
        }
    }
}
