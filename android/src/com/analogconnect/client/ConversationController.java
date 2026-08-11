package com.analogconnect.client;

interface ConversationController {
    interface Gateway {
        ConversationPageData<ConversationSummary> conversations() throws Exception;
        ConversationPageData<ConversationMessage> messages(String conversationId) throws Exception;
    }

    interface View {
        void showLoading();
        void showConversations(ConversationPageData<ConversationSummary> page);
        void showMessages(ConversationSummary conversation,
                ConversationPageData<ConversationMessage> page);
        void showFixedError(String message);
    }

    final class Runner {
        private final Gateway gateway;
        private final View view;

        Runner(Gateway gateway, View view) {
            this.gateway = gateway;
            this.view = view;
        }

        void loadConversations() {
            view.showLoading();
            try {
                view.showConversations(gateway.conversations());
            } catch (Exception error) {
                view.showFixedError("Could not load conversations");
            }
        }

        void openConversation(ConversationSummary conversation) {
            view.showLoading();
            try {
                view.showMessages(conversation, gateway.messages(conversation.id));
            } catch (Exception error) {
                view.showFixedError("Could not load messages");
            }
        }
    }
}
