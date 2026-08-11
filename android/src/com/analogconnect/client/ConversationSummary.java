package com.analogconnect.client;

final class ConversationSummary {
    final String id;
    final String displayAddress;
    final String displayName;
    final boolean group;
    final boolean replySupported;
    final long latestUnixMillis;
    final long messageCount;
    final long unreadCount;
    final String latestOutgoingState;
    final String kind;
    final String title;
    final boolean canReply;
    final boolean identityConflict;
    final String latestPreview;
    final String latestSender;
    final boolean latestSent;

    ConversationSummary(String id, String displayAddress, String displayName, boolean group,
            boolean replySupported, long latestUnixMillis,
            long messageCount, long unreadCount, String latestOutgoingState) {
        this(id, displayAddress, displayName, group, replySupported, latestUnixMillis,
                messageCount, unreadCount, latestOutgoingState, group ? "group" : "private",
                displayName == null ? displayAddress : displayName, replySupported, false);
    }

    ConversationSummary(String id, String displayAddress, String displayName, boolean group,
            boolean replySupported, long latestUnixMillis, long messageCount, long unreadCount,
            String latestOutgoingState, String kind, String title, boolean canReply,
            boolean identityConflict) {
        this(id, displayAddress, displayName, group, replySupported, latestUnixMillis,
                messageCount, unreadCount, latestOutgoingState, kind, title, canReply,
                identityConflict, "", "", false);
    }

    ConversationSummary(String id, String displayAddress, String displayName, boolean group,
            boolean replySupported, long latestUnixMillis, long messageCount, long unreadCount,
            String latestOutgoingState, String kind, String title, boolean canReply,
            boolean identityConflict, String latestPreview, String latestSender,
            boolean latestSent) {
        if (!validConversationId(id) || displayAddress == null
                || displayAddress.isEmpty() || displayAddress.length() > 128
                || latestUnixMillis < 0 || messageCount < 0 || unreadCount < 0
                || unreadCount > messageCount || (group && replySupported)
                || (displayName != null && (displayName.isEmpty() || displayName.length() > 256))
                || title == null || title.isEmpty() || title.length() > 256
                || !("private".equals(kind) || "group".equals(kind)
                        || "ambiguous".equals(kind))
                || group != "group".equals(kind) && !identityConflict
                || canReply != replySupported || (group && canReply) || (identityConflict && canReply)
                || latestPreview == null || latestPreview.length() > 2000
                || latestSender == null || latestSender.length() > 128
                || !validOutgoingState(latestOutgoingState)) {
            throw new IllegalArgumentException("Conversation summary is invalid");
        }
        this.id = id;
        this.displayAddress = displayAddress;
        this.displayName = displayName;
        this.group = group;
        this.replySupported = replySupported;
        this.latestUnixMillis = latestUnixMillis;
        this.messageCount = messageCount;
        this.unreadCount = unreadCount;
        this.latestOutgoingState = latestOutgoingState;
        this.kind = kind;
        this.title = title;
        this.canReply = canReply;
        this.identityConflict = identityConflict;
        this.latestPreview = latestPreview;
        this.latestSender = latestSender;
        this.latestSent = latestSent;
    }

    String displayLabel() {
        return title;
    }

    boolean canUsePrivateReply() {
        return canReply && replySupported && !group && !identityConflict
                && "private".equals(kind);
    }

    String previewLabel() {
        if (latestPreview.isEmpty()) return group ? "Group conversation" : "No messages yet";
        if (latestSent) return "You: " + latestPreview;
        if (group && !latestSender.isEmpty()) return latestSender + ": " + latestPreview;
        return latestPreview;
    }

    static boolean validOutgoingState(String state) {
        return state == null || "queued".equals(state) || "sending".equals(state)
                || "sent_unconfirmed".equals(state) || "sent_confirmed".equals(state)
                || "failed_retryable".equals(state) || "failed_permanent".equals(state)
                || "unknown".equals(state);
    }

    private static boolean validConversationId(String value) {
        if (MessageOperationId.isValid(value)) {
            return true;
        }
        if (value == null || !value.startsWith("ancs-v1-") || value.length() != 72) {
            return false;
        }
        for (int index = 8; index < value.length(); index++) {
            char character = value.charAt(index);
            if (!((character >= '0' && character <= '9')
                    || (character >= 'a' && character <= 'f'))) {
                return false;
            }
        }
        return true;
    }

    @Override public String toString() {
        return "ConversationSummary([private fields redacted])";
    }
}
