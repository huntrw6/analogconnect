package com.analogconnect.client;

public final class ConversationTimeTest {
    public static void main(String[] args) {
        long now = 200000000L;
        require("Now".equals(ConversationTime.label(now - 1000L, now)), "now");
        require("5m".equals(ConversationTime.label(now - 300000L, now)), "minutes");
        require("2h".equals(ConversationTime.label(now - 7200000L, now)), "hours");
        require("Yesterday".equals(ConversationTime.label(now - 90000000L, now)), "yesterday");
        System.out.println("ANDROID_CONVERSATION_TIME_TESTS=PASS tests=4");
    }

    private static void require(boolean condition, String label) {
        if (!condition) throw new AssertionError(label);
    }
}
