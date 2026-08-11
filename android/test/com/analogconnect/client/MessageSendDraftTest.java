package com.analogconnect.client;

public final class MessageSendDraftTest {
    public static void main(String[] args) {
        SequenceGenerator generator = new SequenceGenerator();
        MessageSendDraft draft = new MessageSendDraft(generator);

        String first = draft.operationIdFor("synthetic-recipient-a", "synthetic-body-a");
        require(first.equals(draft.operationIdFor(
                "synthetic-recipient-a", "synthetic-body-a")), "unchanged retry reuses ID");
        require(!first.equals(draft.operationIdFor(
                "synthetic-recipient-a", "synthetic-body-b")), "body change rotates ID");
        String beforeAccepted = draft.operationIdFor(
                "synthetic-recipient-a", "synthetic-body-b");
        draft.accepted();
        require(!beforeAccepted.equals(draft.operationIdFor(
                "synthetic-recipient-a", "synthetic-body-b")), "acceptance clears ID");
        require(generator.count == 3, "generator called only for new operations");
        System.out.println("ANDROID_MESSAGE_DRAFT_TESTS=PASS tests=4");
    }

    private static void require(boolean condition, String label) {
        if (!condition) {
            throw new AssertionError(label);
        }
    }

    private static final class SequenceGenerator implements MessageSendDraft.IdGenerator {
        int count;

        @Override public String generate() {
            count++;
            String suffix = Integer.toHexString(count);
            StringBuilder value = new StringBuilder();
            while (value.length() < 32 - suffix.length()) {
                value.append('0');
            }
            return value.append(suffix).toString();
        }
    }
}
