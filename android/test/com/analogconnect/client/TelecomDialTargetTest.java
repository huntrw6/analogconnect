package com.analogconnect.client;

public final class TelecomDialTargetTest {
    public static void main(String[] args) {
        assertEquals("+12025550101", TelecomDialTarget.validate(" +12025550101 "));
        assertEquals("*86", TelecomDialTarget.validate("*86"));
        reject("");
        reject("not a number");
        reject("123,456");
        reject("123456789012345678901234567890123");
        System.out.println("ANDROID_TELECOM_TARGET_TESTS=PASS tests=6");
    }

    private static void reject(String value) {
        try {
            TelecomDialTarget.validate(value);
            throw new AssertionError("expected invalid target");
        } catch (IllegalArgumentException expected) {
            // Expected; diagnostics must never echo the target.
            if (expected.getMessage().contains(value) && !value.isEmpty()) {
                throw new AssertionError("target leaked into diagnostic");
            }
        }
    }

    private static void assertEquals(String expected, String actual) {
        if (!expected.equals(actual)) {
            throw new AssertionError("target mismatch");
        }
    }
}
