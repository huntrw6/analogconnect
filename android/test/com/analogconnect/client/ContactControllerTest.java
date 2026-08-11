package com.analogconnect.client;

import java.util.Arrays;

public final class ContactControllerTest {
    public static void main(String[] args) {
        FixtureGateway gateway = new FixtureGateway();
        FixtureView view = new FixtureView();
        ContactController.Runner runner = new ContactController.Runner(gateway, view);
        runner.load(" Example ", "0000000000000001");
        require("Example".equals(gateway.query), "query normalized");
        require("0000000000000001".equals(gateway.cursor), "cursor retained");
        require(view.loading == 1 && view.items == 1, "successful transition");
        gateway.fail = true;
        runner.load("private-query", null);
        require(view.failures == 1, "fixed failure");
        System.out.println("ANDROID_CONTACT_CONTROLLER_TESTS=PASS tests=4");
    }

    private static final class FixtureGateway implements ContactController.Gateway {
        String query;
        String cursor;
        boolean fail;

        @Override public ConversationPageData<ContactListItem> contacts(String value, String page)
                throws Exception {
            query = value;
            cursor = page;
            if (fail) {
                throw new Exception("private-backend-value");
            }
            return new ConversationPageData<ContactListItem>(Arrays.asList(
                    new ContactListItem("Example", Arrays.asList("synthetic-number"))), null);
        }
    }

    private static final class FixtureView implements ContactController.View {
        int loading;
        int items;
        int failures;

        @Override public void showLoading() { loading++; }
        @Override public void showContacts(ConversationPageData<ContactListItem> page) {
            items = page.items.size();
        }
        @Override public void showFixedError() { failures++; }
    }

    private static void require(boolean condition, String label) {
        if (!condition) {
            throw new AssertionError(label);
        }
    }
}
