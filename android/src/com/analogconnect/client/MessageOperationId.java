package com.analogconnect.client;

import java.security.SecureRandom;

final class MessageOperationId {
    private static final int BYTE_COUNT = 16;
    private static final char[] HEX = "0123456789abcdef".toCharArray();

    private MessageOperationId() {}

    static String generate() {
        return generate(new SecureRandom());
    }

    static String generate(SecureRandom random) {
        byte[] bytes = new byte[BYTE_COUNT];
        random.nextBytes(bytes);
        char[] encoded = new char[BYTE_COUNT * 2];
        for (int index = 0; index < bytes.length; index++) {
            int value = bytes[index] & 0xff;
            encoded[index * 2] = HEX[value >>> 4];
            encoded[index * 2 + 1] = HEX[value & 0x0f];
        }
        return new String(encoded);
    }

    static boolean isValid(String value) {
        if (value == null || value.length() != BYTE_COUNT * 2) {
            return false;
        }
        for (int index = 0; index < value.length(); index++) {
            char character = value.charAt(index);
            if (!((character >= '0' && character <= '9')
                    || (character >= 'a' && character <= 'f'))) {
                return false;
            }
        }
        return true;
    }
}
