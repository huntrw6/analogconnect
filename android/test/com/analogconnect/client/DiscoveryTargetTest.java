package com.analogconnect.client;

import java.net.InetAddress;

public final class DiscoveryTargetTest {
    public static void main(String[] args) throws Exception {
        DiscoveryTarget ipv4 = DiscoveryTarget.from(InetAddress.getByAddress(
                "operat.local", new byte[] {(byte) 192, 0, 2, 10}), 8787);
        assertEquals("https://192.0.2.10:8787", ipv4.endpoint);
        assertEquals("operat.local", ipv4.tlsName);

        byte[] v6 = new byte[16];
        v6[15] = 1;
        DiscoveryTarget ipv6 = DiscoveryTarget.from(InetAddress.getByAddress("operat.local", v6), 443);
        assertEquals("https://[0:0:0:0:0:0:0:1]:443", ipv6.endpoint);
        assertRejected(InetAddress.getByAddress(new byte[] {127, 0, 0, 1}), 8787);
        assertRejected(InetAddress.getByAddress("operat.local", new byte[] {127, 0, 0, 1}), 0);
        DiscoveryTarget txtIdentity = DiscoveryTarget.from(
                InetAddress.getByAddress(new byte[] {(byte) 192, 0, 2, 11}), 8787, "operat.local");
        assertEquals("operat.local", txtIdentity.tlsName);
        System.out.println("ANDROID_DISCOVERY_TESTS=PASS tests=7");
    }

    private static void assertRejected(InetAddress host, int port) {
        try {
            DiscoveryTarget.from(host, port);
            throw new AssertionError("expected discovery rejection");
        } catch (IllegalArgumentException expected) {
            // Fixed error only.
        }
    }

    private static void assertEquals(String expected, String actual) {
        if (!expected.equals(actual)) throw new AssertionError("discovery target mismatch");
    }
}
