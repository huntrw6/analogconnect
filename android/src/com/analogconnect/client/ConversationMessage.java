package com.analogconnect.client;

final class ConversationMessage {
    final String id;
    final long timestampUnixMillis;
    final boolean sent;
    final String peerAddress;
    final String body;
    final boolean read;
    final String outgoingState;

    ConversationMessage(String id, long timestampUnixMillis, String direction,
            String peerAddress, String body, boolean read, String outgoingState) {
        if (id == null || id.length() != 16 || !lowerHex(id) || timestampUnixMillis < 0
                || !("received".equals(direction) || "sent".equals(direction))
                || peerAddress == null || peerAddress.length() > 128
                || body == null || body.length() > 2000
                || !ConversationSummary.validOutgoingState(outgoingState)) {
            throw new IllegalArgumentException("Conversation message is invalid");
        }
        this.id = id;
        this.timestampUnixMillis = timestampUnixMillis;
        this.sent = "sent".equals(direction);
        this.peerAddress = peerAddress;
        this.body = body;
        this.read = read;
        this.outgoingState = outgoingState;
    }

    private static boolean lowerHex(String value) {
        for (int index = 0; index < value.length(); index++) {
            char character = value.charAt(index);
            if (!((character >= '0' && character <= '9')
                    || (character >= 'a' && character <= 'f'))) {
                return false;
            }
        }
        return true;
    }

    @Override public String toString() {
        return "ConversationMessage([private fields redacted])";
    }
}
