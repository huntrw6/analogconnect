package com.analogconnect.client;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/** Detects only newly observed incoming/unread conversation revisions. */
final class MessageNotificationTracker {
    private final Map<String, Long> latest = new HashMap<String, Long>();
    private boolean initialized;

    List<ConversationSummary> update(List<ConversationSummary> conversations) {
        List<ConversationSummary> changed = new ArrayList<ConversationSummary>();
        for (ConversationSummary item : conversations) {
            Long previous = latest.put(item.id, Long.valueOf(item.latestUnixMillis));
            if (initialized && item.unreadCount > 0 && !item.latestSent
                    && (previous == null || item.latestUnixMillis > previous.longValue())) {
                changed.add(item);
            }
        }
        initialized = true;
        return changed;
    }

    static int notificationId(String conversationId) {
        return 10_000 + (conversationId.hashCode() & 0x3fffffff);
    }
}
