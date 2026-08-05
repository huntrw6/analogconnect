package com.analogconnect.client;

final class TelecomDialTarget {
    private TelecomDialTarget() {}

    static String validate(String candidate) {
        String target = candidate == null ? "" : candidate.trim();
        if (target.isEmpty() || target.length() > 32) {
            throw new IllegalArgumentException("Dial target is invalid");
        }
        for (int index = 0; index < target.length(); index++) {
            char value = target.charAt(index);
            if (!(Character.isDigit(value) || value == '+' || value == '*' || value == '#')) {
                throw new IllegalArgumentException("Dial target is invalid");
            }
        }
        return target;
    }
}
