package com.analogconnect.client;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Locale;

final class DemoFixtures {
    private DemoFixtures() {}

    static ConversationPageData<ConversationSummary> conversations() {
        long now = System.currentTimeMillis();
        List<ConversationSummary> items = Arrays.asList(
                direct("11111111111111111111111111111111", "Jordan Lee", now, 2, "sent_confirmed"),
                new ConversationSummary(groupId('a'), "group", null, true, false,
                        now - 3600000L, 8, 3, null, "group", "Weekend plans", false, false,
                        "That works for me.", "Riley", false),
                new ConversationSummary(groupId('b'), "group", null, true, false,
                        now - 86400000L, 4, 0, null, "group", "To you, Sam & Riley", false, false,
                        "See you tomorrow!", "Sam", false),
                direct("22222222222222222222222222222222", "Casey Morgan",
                        now - 172800000L, 0, "failed_retryable"),
                new ConversationSummary("33333333333333333333333333333333", "private", null,
                        false, false, now - 259200000L, 2, 1, null, "ambiguous",
                        "Conversation needs attention", false, true, "Identity needs review",
                        "", false));
        return new ConversationPageData<ConversationSummary>(items, null);
    }

    static ConversationPageData<ConversationMessage> messages(ConversationSummary conversation) {
        long now = System.currentTimeMillis();
        List<ConversationMessage> items = new ArrayList<ConversationMessage>();
        if (conversation.group) {
            items.add(message("aaaaaaaaaaaaaaaa", now - 60000L, "received", "Riley",
                    "That works for me.", false, null));
            items.add(message("bbbbbbbbbbbbbbbb", now - 120000L, "sent", "",
                    "How about Saturday morning?", true, "sent_confirmed"));
            items.add(message("cccccccccccccccc", now - 180000L, "received", "Sam",
                    "Want to meet at the usual place?", true, null));
        } else {
            items.add(message("dddddddddddddddd", now - 60000L, "received", "",
                    "Sounds good — see you then!", false, null));
            items.add(message("eeeeeeeeeeeeeeee", now - 180000L, "sent", "",
                    "I'll be there around six.", true, conversation.latestOutgoingState));
        }
        return new ConversationPageData<ConversationMessage>(items, null);
    }

    static ConversationPageData<ContactListItem> contacts(String query) {
        List<ContactListItem> all = Arrays.asList(
                new ContactListItem("Alex Rivera", Arrays.asList("5550101", "5550102")),
                new ContactListItem("Casey Morgan", Arrays.asList("5550103")),
                new ContactListItem("Jordan Lee", Arrays.asList("5550104")),
                new ContactListItem(null, Arrays.asList("5550105")),
                new ContactListItem("Riley Chen", Arrays.asList("5550106")),
                new ContactListItem("Sam Patel", Arrays.asList("5550107")));
        String needle = query == null ? "" : query.trim().toLowerCase(Locale.ROOT);
        List<ContactListItem> result = new ArrayList<ContactListItem>();
        for (ContactListItem item : all) {
            if (needle.isEmpty() || item.displayName != null
                    && item.displayName.toLowerCase(Locale.ROOT).contains(needle)) result.add(item);
        }
        return new ConversationPageData<ContactListItem>(result, null);
    }

    private static ConversationSummary direct(String id, String title, long time, long unread,
            String state) {
        return new ConversationSummary(id, "5550199", title, false, true, time,
                Math.max(2, unread), unread, state, "private", title, true, false,
                state != null && state.startsWith("failed") ? "Message wasn't sent"
                        : "Sounds good — see you then!", "", state == null || !state.startsWith("failed"));
    }

    private static ConversationMessage message(String id, long time, String direction,
            String peer, String body, boolean read, String state) {
        return new ConversationMessage(id, time, direction, peer, body, read, state);
    }

    private static String groupId(char value) {
        StringBuilder result = new StringBuilder("ancs-v1-");
        for (int index = 0; index < 64; index++) result.append(value);
        return result.toString();
    }
}
