package com.analogconnect.client;

public final class DemoFixturesTest {
    public static void main(String[] args) {
        ConversationPageData<ConversationSummary> conversations = DemoFixtures.conversations();
        require(conversations.items.size() >= 5, "realistic conversation set");
        ConversationSummary group = conversations.items.get(1);
        require(group.group && "Weekend plans".equals(group.displayLabel()), "named group title");
        require(!group.canUsePrivateReply(), "group reply closed");
        ConversationPageData<ConversationMessage> messages = DemoFixtures.messages(group);
        require(messages.items.size() == 3, "group messages");
        require(!messages.items.get(0).peerAddress.equals(messages.items.get(2).peerAddress),
                "different group senders");
        require(DemoFixtures.contacts("jordan").items.size() == 1, "contact search");
        require(DemoFixtures.contacts("missing").items.isEmpty(), "contact empty state");
        System.out.println("ANDROID_DEMO_FIXTURE_TESTS=PASS tests=7");
    }

    private static void require(boolean condition, String label) {
        if (!condition) throw new AssertionError(label);
    }
}
