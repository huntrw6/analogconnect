package com.analogconnect.client;

import java.net.MalformedURLException;

public final class EndpointTest {
    public static void main(String[] args) throws Exception {
        assertEquals("http://127.0.0.1:8787/api/v1/health",
                Endpoint.parse(" http://127.0.0.1:8787 ", "/api/v1/health").toString());
        assertEquals("https://pi.local/base/api/v1/status",
                Endpoint.parse("https://pi.local/base", "api/v1/status").toString());
        assertRejected("file:///tmp/socket");
        assertRejected("http://user:secret@pi.local");
        assertRejected("http:///missing-host");
        System.out.println("ANDROID_UNIT_TESTS=PASS tests=5");
    }

    private static void assertRejected(String value) throws Exception {
        try {
            Endpoint.parse(value, "/api/v1/health");
            throw new AssertionError("expected endpoint rejection");
        } catch (MalformedURLException expected) {
            // Expected. Do not include the supplied value in output.
        }
    }

    private static void assertEquals(String expected, String actual) {
        if (!expected.equals(actual)) {
            throw new AssertionError("endpoint did not match expected value");
        }
    }
}
