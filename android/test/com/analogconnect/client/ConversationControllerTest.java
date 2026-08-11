package com.analogconnect.client;

import java.util.Arrays;
import java.util.Collections;

public final class ConversationControllerTest {
    public static void main(String[] args) {
        ConversationSummary summary = new ConversationSummary(
                "01010101010101010101010101010101", "synthetic-address", "Example", false, true,
                1, 1, 0, null);
        FixtureGateway gateway = new FixtureGateway(summary);
        FixtureView view = new FixtureView();
        ConversationController.Runner controller = new ConversationController.Runner(gateway, view);

        controller.loadConversations();
        require(view.loadingCount == 1 && view.conversationCount == 1, "list transition");
        controller.openConversation(summary);
        require(view.loadingCount == 2 && view.messageCount == 1, "thread transition");
        gateway.fail = true;
        controller.loadConversations();
        require("Could not load conversations".equals(view.error), "fixed list error");
        controller.openConversation(summary);
        require("Could not load messages".equals(view.error), "fixed thread error");
        require(!view.error.contains("private"), "errors omit private backend data");
        System.out.println("ANDROID_CONVERSATION_CONTROLLER_TESTS=PASS tests=5");
    }

    private static final class FixtureGateway implements ConversationController.Gateway {
        final ConversationSummary summary;
        boolean fail;

        FixtureGateway(ConversationSummary summary) {
            this.summary = summary;
        }

        @Override public ConversationPageData<ConversationSummary> conversations() throws Exception {
            if (fail) {
                throw new Exception("synthetic-private-backend-error");
            }
            return new ConversationPageData<ConversationSummary>(Arrays.asList(summary), null);
        }

        @Override public ConversationPageData<ConversationMessage> messages(String id)
                throws Exception {
            if (fail) {
                throw new Exception("synthetic-private-backend-error");
            }
            return new ConversationPageData<ConversationMessage>(
                    Collections.singletonList(new ConversationMessage(
                            "0000000000000001", 1, "received", "synthetic-address",
                            "synthetic-body", false, null)),
                    null);
        }
    }

    private static final class FixtureView implements ConversationController.View {
        int loadingCount;
        int conversationCount;
        int messageCount;
        String error = "";

        @Override public void showLoading() {
            loadingCount++;
        }

        @Override public void showConversations(ConversationPageData<ConversationSummary> page) {
            conversationCount = page.items.size();
        }

        @Override public void showMessages(ConversationSummary conversation,
                ConversationPageData<ConversationMessage> page) {
            messageCount = page.items.size();
        }

        @Override public void showFixedError(String message) {
            error = message;
        }
    }

    private static void require(boolean condition, String label) {
        if (!condition) {
            throw new AssertionError(label);
        }
    }
}
