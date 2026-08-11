package com.analogconnect.client;

import java.security.SecureRandom;

public final class MessageOperationIdTest {
    public static void main(String[] args) {
        byte[] source = new byte[16];
        for (int index = 0; index < source.length; index++) {
            source[index] = (byte) index;
        }
        String generated = MessageOperationId.generate(new FixedRandom(source));
        require(generated.equals("000102030405060708090a0b0c0d0e0f"), "hex encoding");
        require(MessageOperationId.isValid(generated), "generated identifier validates");
        require(!MessageOperationId.isValid(null), "null rejected");
        require(!MessageOperationId.isValid("00"), "short identifier rejected");
        require(!MessageOperationId.isValid("000102030405060708090A0B0C0D0E0F"),
                "non-canonical uppercase rejected");
        System.out.println("ANDROID_MESSAGE_OPERATION_TESTS=PASS tests=5");
    }

    private static void require(boolean condition, String label) {
        if (!condition) {
            throw new AssertionError(label);
        }
    }

    private static final class FixedRandom extends SecureRandom {
        private final byte[] source;

        FixedRandom(byte[] source) {
            this.source = source.clone();
        }

        @Override public void nextBytes(byte[] bytes) {
            System.arraycopy(source, 0, bytes, 0, bytes.length);
        }
    }
}
