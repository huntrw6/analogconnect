package com.analogconnect.client;

import java.text.SimpleDateFormat;
import java.util.Date;
import java.util.Locale;

final class ConversationTime {
    private ConversationTime() {}

    static String label(long timestamp, long now) {
        long age = Math.max(0L, now - timestamp);
        if (age < 60000L) return "Now";
        if (age < 3600000L) return (age / 60000L) + "m";
        if (age < 86400000L) return (age / 3600000L) + "h";
        if (age < 172800000L) return "Yesterday";
        return new SimpleDateFormat("M/d", Locale.getDefault()).format(new Date(timestamp));
    }
}
