package com.analogconnect.client;

import java.security.GeneralSecurityException;
import java.security.MessageDigest;

public final class CertificatePinTest {
    public static void main(String[] args) throws Exception {
        parsesAndMatchesConstantTimeDigest();
        acceptsColonSeparatedUppercase();
        rejectsMalformedPins();
        redactsDiagnostics();
        System.out.println("ANDROID_PIN_TESTS=PASS tests=4");
    }

    private static void parsesAndMatchesConstantTimeDigest() throws Exception {
        byte[] certificate = "synthetic certificate bytes".getBytes("UTF-8");
        CertificatePin pin = CertificatePin.parse(hex(
                MessageDigest.getInstance("SHA-256").digest(certificate), false));
        assertTrue(pin.matchesEncoded(certificate));
        assertTrue(!pin.matchesEncoded("different".getBytes("UTF-8")));
    }

    private static void acceptsColonSeparatedUppercase() throws Exception {
        byte[] digest = MessageDigest.getInstance("SHA-256").digest(new byte[] {1, 2, 3});
        CertificatePin.parse(hex(digest, true));
    }

    private static void rejectsMalformedPins() throws Exception {
        for (String value : new String[] {"", "abcd", repeat('z', 64)}) {
            try {
                CertificatePin.parse(value);
                throw new AssertionError("expected certificate pin rejection");
            } catch (GeneralSecurityException expected) {
                // Expected. Input is not echoed.
            }
        }
    }

    private static void redactsDiagnostics() throws Exception {
        String value = repeat('a', 64);
        String debug = CertificatePin.parse(value).toString();
        assertTrue(!debug.contains(value));
        assertTrue("CertificatePin([redacted])".equals(debug));
    }

    private static String hex(byte[] bytes, boolean colonSeparated) {
        StringBuilder value = new StringBuilder();
        for (int index = 0; index < bytes.length; index++) {
            if (colonSeparated && index > 0) {
                value.append(':');
            }
            value.append(String.format("%02X", bytes[index] & 0xff));
        }
        return value.toString();
    }

    private static String repeat(char value, int count) {
        StringBuilder result = new StringBuilder(count);
        for (int index = 0; index < count; index++) {
            result.append(value);
        }
        return result.toString();
    }

    private static void assertTrue(boolean condition) {
        if (!condition) {
            throw new AssertionError("certificate pin assertion failed");
        }
    }
}
