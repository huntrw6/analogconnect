package com.analogconnect.client;

import java.util.Arrays;
import java.util.Collections;

public final class MessageNotificationTrackerTest {
    public static void main(String[] args) {
        MessageNotificationTracker tracker = new MessageNotificationTracker();
        ConversationSummary direct = summary("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", false,
                100, 1, false, "Hello", "");
        require(tracker.update(Collections.singletonList(direct)).isEmpty(),
                "initial snapshot must not notify historical messages");
        ConversationSummary newer = summary("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", false,
                101, 2, false, "Again", "");
        require(tracker.update(Collections.singletonList(newer)).size() == 1,
                "new unread incoming revision must notify");
        require(tracker.update(Collections.singletonList(newer)).isEmpty(),
                "same revision must not notify twice");

        ConversationSummary group = summary(groupId(), true, 102, 1, false, "Hi", "Person");
        ConversationSummary sent = summary("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", false,
                103, 0, true, "Sent", "");
        require(tracker.update(Arrays.asList(group, sent)).size() == 1,
                "group incoming notifies while sent messages do not");
        require(group.previewLabel().equals("Person: Hi"), "group preview includes sender");
        require(MessageNotificationTracker.notificationId(group.id)
                        == MessageNotificationTracker.notificationId(group.id),
                "notification IDs must be stable per conversation");
    }

    private static ConversationSummary summary(String id, boolean group, long time, long unread,
            boolean sent, String preview, String sender) {
        return new ConversationSummary(id, "synthetic", null, group, false, time,
                Math.max(1, unread), unread, null, group ? "group" : "private",
                group ? "Synthetic Group" : "Synthetic Person", false, false,
                preview, sender, sent);
    }

    private static String groupId() {
        StringBuilder id = new StringBuilder("ancs-v1-");
        while (id.length() < 72) id.append('a');
        return id.toString();
    }

    private static void require(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }
}
