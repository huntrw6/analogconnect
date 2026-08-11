package com.analogconnect.client;

import java.util.Arrays;

public final class ConversationModelTest {
    public static void main(String[] args) {
        String privateAddress = "synthetic-private-address";
        String privateBody = "synthetic-private-body";
        ConversationSummary summary = new ConversationSummary(
                "01010101010101010101010101010101", privateAddress, "Example Contact", false, true,
                1, 2, 1, null);
        ConversationMessage message = new ConversationMessage(
                "0000000000000001", 1, "received", privateAddress,
                privateBody, false, null);
        require(!summary.toString().contains(privateAddress), "summary redacts address");
        require(!message.toString().contains(privateBody), "message redacts body");
        ConversationSummary group = new ConversationSummary(
                "02020202020202020202020202020202", "synthetic-a, synthetic-b, synthetic-c", null,
                true, false, 2, 3, 0, null);
        require(group.group && !group.replySupported, "group replies fail closed");
        String ancsId = "ancs-v1-"
                + "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        ConversationSummary ancsGroup = new ConversationSummary(
                ancsId, "synthetic-latest-sender", null, true, false, 3, 4, 0, null,
                "group", "Synthetic Group Title", false, false);
        require(ancsGroup.displayLabel().equals("Synthetic Group Title"),
                "ANCS title is displayed");
        require(!ancsGroup.canUsePrivateReply(), "ANCS group cannot use private reply");
        ConversationSummary conflict = new ConversationSummary(
                ancsId, "synthetic-latest-sender", null, true, false, 3, 4, 0, null,
                "ambiguous", "Synthetic Conflict", false, true);
        require(!conflict.canUsePrivateReply(), "conflict cannot use private reply");
        require(summary.canUsePrivateReply(), "private reply remains enabled");
        ConversationPageData<ConversationSummary> page = new ConversationPageData<ConversationSummary>(
                Arrays.asList(summary), "0000000000000001");
        require(page.items.size() == 1, "page retains items");
        rejectSummary("bad", privateAddress, 1, 1, 0, null);
        rejectSummary("01010101010101010101010101010101", privateAddress, 1, 1, 2, null);
        rejectMessage("bad", 1, "received", privateBody, false, null);
        rejectMessage("0000000000000001", 1, "sideways", privateBody, false, null);
        System.out.println("ANDROID_CONVERSATION_MODEL_TESTS=PASS tests=12");
    }

    private static void rejectSummary(String id, String address, long latest, long count,
            long unread, String state) {
        try {
            new ConversationSummary(id, address, null, false, true, latest, count, unread, state);
            throw new AssertionError("invalid summary accepted");
        } catch (IllegalArgumentException expected) {
            // Expected.
        }
    }

    private static void rejectMessage(String id, long timestamp, String direction, String body,
            boolean read, String state) {
        try {
            new ConversationMessage(id, timestamp, direction, "synthetic-peer", body, read, state);
            throw new AssertionError("invalid message accepted");
        } catch (IllegalArgumentException expected) {
            // Expected.
        }
    }

    private static void require(boolean condition, String label) {
        if (!condition) {
            throw new AssertionError(label);
        }
    }
}
