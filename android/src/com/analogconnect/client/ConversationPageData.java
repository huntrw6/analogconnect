package com.analogconnect.client;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

final class ConversationPageData<T> {
    final List<T> items;
    final String nextCursor;

    ConversationPageData(List<T> items, String nextCursor) {
        if (items == null || items.size() > 100
                || (nextCursor != null && (nextCursor.length() != 16 || !lowerHex(nextCursor)))) {
            throw new IllegalArgumentException("Conversation page is invalid");
        }
        this.items = Collections.unmodifiableList(new ArrayList<T>(items));
        this.nextCursor = nextCursor;
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
}
