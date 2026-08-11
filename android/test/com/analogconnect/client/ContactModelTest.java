package com.analogconnect.client;

import java.util.Arrays;

public final class ContactModelTest {
    public static void main(String[] args) {
        ContactListItem item = new ContactListItem(
                "Example Contact", Arrays.asList("synthetic-number"));
        require(item.labelFor("synthetic-number").startsWith("Example Contact"), "name label");
        require(!item.toString().contains("Example Contact"), "debug redacts name");
        require(!item.toString().contains("synthetic-number"), "debug redacts number");
        ContactListItem unnamed = new ContactListItem(null, Arrays.asList("synthetic-number"));
        require("synthetic-number".equals(unnamed.labelFor("synthetic-number")),
                "unnamed fallback");
        reject(null);
        System.out.println("ANDROID_CONTACT_MODEL_TESTS=PASS tests=5");
    }

    private static void reject(java.util.List<String> phones) {
        try {
            new ContactListItem("Example", phones);
            throw new AssertionError("invalid contact accepted");
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
