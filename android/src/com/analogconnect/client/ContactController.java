package com.analogconnect.client;

interface ContactController {
    interface Gateway {
        ConversationPageData<ContactListItem> contacts(String query, String cursor) throws Exception;
    }

    interface View {
        void showLoading();
        void showContacts(ConversationPageData<ContactListItem> page);
        void showFixedError();
    }

    final class Runner {
        private final Gateway gateway;
        private final View view;

        Runner(Gateway gateway, View view) {
            this.gateway = gateway;
            this.view = view;
        }

        void load(String query, String cursor) {
            view.showLoading();
            try {
                view.showContacts(gateway.contacts(
                        query == null ? "" : query.trim(), cursor));
            } catch (Exception ignored) {
                view.showFixedError();
            }
        }
    }
}
