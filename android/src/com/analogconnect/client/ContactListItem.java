package com.analogconnect.client;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

final class ContactListItem {
    final String displayName;
    final List<String> phoneNumbers;

    ContactListItem(String displayName, List<String> phoneNumbers) {
        if (displayName != null && (displayName.isEmpty() || displayName.length() > 256)) {
            throw new IllegalArgumentException("Contact name is invalid");
        }
        if (phoneNumbers == null || phoneNumbers.isEmpty() || phoneNumbers.size() > 32) {
            throw new IllegalArgumentException("Contact phone list is invalid");
        }
        ArrayList<String> copy = new ArrayList<String>(phoneNumbers.size());
        for (String number : phoneNumbers) {
            if (number == null || number.isEmpty() || number.length() > 128) {
                throw new IllegalArgumentException("Contact phone number is invalid");
            }
            copy.add(number);
        }
        this.displayName = displayName;
        this.phoneNumbers = Collections.unmodifiableList(copy);
    }

    String labelFor(String number) {
        return displayName == null ? number : displayName + "\n" + number;
    }

    @Override public String toString() {
        return "ContactListItem([private fields redacted])";
    }
}
